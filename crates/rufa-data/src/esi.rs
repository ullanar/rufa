use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::Duration;

use ring::digest;
use ring::rand::{SecureRandom, SystemRandom};

const AUTHORIZE: &str = "https://login.eveonline.com/v2/oauth/authorize/";
const TOKEN: &str = "https://login.eveonline.com/v2/oauth/token";
const ESI: &str = "https://esi.evetech.net/latest";
pub const SCOPES: &str = "esi-skills.read_skills.v1";
const COMPATIBILITY_DATE: &str = "2026-08-18";

#[derive(Debug, Clone, PartialEq)]
pub struct Pilot {
    pub character_id: i64,
    pub name: String,
    pub skills: BTreeMap<i32, u8>,
}

pub fn base64url(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        if chunk.len() > 1 {
            out.push(ALPHABET[(n >> 6) as usize & 63] as char);
        }
        if chunk.len() > 2 {
            out.push(ALPHABET[n as usize & 63] as char);
        }
    }
    out
}

fn base64url_decode(text: &str) -> Option<Vec<u8>> {
    let value = |c: u8| -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => (c - b'A') as u32,
            b'a'..=b'z' => (c - b'a') as u32 + 26,
            b'0'..=b'9' => (c - b'0') as u32 + 52,
            b'-' | b'+' => 62,
            b'_' | b'/' => 63,
            _ => return None,
        })
    };
    let bytes: Vec<u8> = text.bytes().filter(|c| *c != b'=').collect();
    let mut out = Vec::new();
    for chunk in bytes.chunks(4) {
        let mut n: u32 = 0;
        for (i, c) in chunk.iter().enumerate() {
            n |= value(*c)? << (18 - 6 * i);
        }
        out.push((n >> 16) as u8);
        if chunk.len() > 2 {
            out.push((n >> 8) as u8);
        }
        if chunk.len() > 3 {
            out.push(n as u8);
        }
    }
    Some(out)
}

pub fn challenge(verifier: &str) -> String {
    base64url(digest::digest(&digest::SHA256, verifier.as_bytes()).as_ref())
}

fn random_token() -> String {
    let mut bytes = [0u8; 32];
    SystemRandom::new()
        .fill(&mut bytes)
        .expect("system randomness");
    base64url(&bytes)
}

fn percent_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(60)))
        .user_agent("rufa (https://github.com/EVEShipFit)")
        .build()
        .new_agent()
}

pub fn character_id_of(access_token: &str) -> Option<i64> {
    let payload = access_token.split('.').nth(1)?;
    let json = base64url_decode(payload)?;
    let value: serde_json::Value = serde_json::from_slice(&json).ok()?;
    value.get("sub")?.as_str()?.rsplit(':').next()?.parse().ok()
}

pub fn login(client_id: &str) -> Result<Pilot, String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let redirect = format!("http://localhost:{port}/callback");
    let verifier = random_token();
    let state = random_token();
    let url = format!(
        "{AUTHORIZE}?response_type=code&redirect_uri={}&client_id={}&scope={}&code_challenge={}&code_challenge_method=S256&state={}",
        percent_encode(&redirect),
        percent_encode(client_id),
        percent_encode(SCOPES),
        challenge(&verifier),
        state
    );
    open_browser(&url)?;

    listener.set_nonblocking(false).ok();
    let (mut stream, _) = listener.accept().map_err(|e| e.to_string())?;
    stream.set_read_timeout(Some(Duration::from_secs(10))).ok();
    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf).map_err(|e| e.to_string())?;
    let request = String::from_utf8_lossy(&buf[..n]).to_string();
    let code = query_param(&request, "code").ok_or("no code in the callback")?;
    let got_state = query_param(&request, "state").unwrap_or_default();
    let body = if got_state == state {
        "<html><body style=\"font-family:sans-serif\">Rufa received the login. You can close this tab.</body></html>"
    } else {
        "<html><body>State mismatch, try again from Rufa.</body></html>"
    };
    let _ = write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    if got_state != state {
        return Err("state mismatch in the SSO callback".to_string());
    }

    let form = format!(
        "grant_type=authorization_code&code={}&client_id={}&code_verifier={}",
        percent_encode(&code),
        percent_encode(client_id),
        percent_encode(&verifier)
    );
    let mut response = agent()
        .post(TOKEN)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Host", "login.eveonline.com")
        .send(form.as_bytes())
        .map_err(|e| format!("token exchange: {e}"))?;
    let text = response
        .body_mut()
        .read_to_string()
        .map_err(|e| e.to_string())?;
    let token: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let access = token
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or("no access token")?;
    let character_id = character_id_of(access).ok_or("no character in the token")?;
    let name = fetch_name(character_id)?;
    let skills = fetch_skills(character_id, access)?;
    Ok(Pilot {
        character_id,
        name,
        skills,
    })
}

fn query_param(request: &str, key: &str) -> Option<String> {
    let line = request.lines().next()?;
    let path = line.split_whitespace().nth(1)?;
    let query = path.split_once('?')?.1;
    query.split('&').find_map(|pair| {
        pair.split_once('=')
            .filter(|(k, _)| *k == key)
            .map(|(_, v)| v.to_string())
    })
}

fn open_browser(url: &str) -> Result<(), String> {
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "windows") {
        "explorer"
    } else {
        "xdg-open"
    };
    std::process::Command::new(opener)
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("cannot open a browser with {opener}: {e}"))
}

fn fetch_name(character_id: i64) -> Result<String, String> {
    let mut response = agent()
        .get(format!("{ESI}/characters/{character_id}/"))
        .header("X-Compatibility-Date", COMPATIBILITY_DATE)
        .call()
        .map_err(|e| e.to_string())?;
    let text = response
        .body_mut()
        .read_to_string()
        .map_err(|e| e.to_string())?;
    let value: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok(value
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Pilot")
        .to_string())
}

pub fn fetch_skills(character_id: i64, access_token: &str) -> Result<BTreeMap<i32, u8>, String> {
    let mut response = agent()
        .get(format!("{ESI}/characters/{character_id}/skills/"))
        .header("X-Compatibility-Date", COMPATIBILITY_DATE)
        .header("Authorization", format!("Bearer {access_token}"))
        .call()
        .map_err(|e| e.to_string())?;
    let text = response
        .body_mut()
        .read_to_string()
        .map_err(|e| e.to_string())?;
    parse_skills(&text)
}

pub fn parse_skills(json: &str) -> Result<BTreeMap<i32, u8>, String> {
    let value: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let list = value
        .get("skills")
        .and_then(|v| v.as_array())
        .ok_or("no skills array")?;
    let mut out = BTreeMap::new();
    for skill in list {
        let id = skill.get("skill_id").and_then(|v| v.as_i64());
        let level = skill.get("trained_skill_level").and_then(|v| v.as_i64());
        if let (Some(id), Some(level)) = (id, level)
            && level > 0
        {
            out.insert(id as i32, level.min(5) as u8);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_challenge_matches_rfc_7636() {
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        assert_eq!(
            challenge(verifier),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
        assert_eq!(base64url(b"hi"), "aGk");
        assert_eq!(base64url_decode("aGk").as_deref(), Some(&b"hi"[..]));
    }

    #[test]
    fn character_id_comes_from_the_sub_claim() {
        let payload = base64url(br#"{"sub":"CHARACTER:EVE:95465499","name":"Pilot"}"#);
        let token = format!("eyJhbGciOiJSUzI1NiJ9.{payload}.signature");
        assert_eq!(character_id_of(&token), Some(95465499));
    }

    #[test]
    fn skills_json_parses() {
        let json = r#"{"skills":[{"skill_id":3300,"trained_skill_level":5,"active_skill_level":5},
                                  {"skill_id":3301,"trained_skill_level":0}],"total_sp":1}"#;
        let skills = parse_skills(json).unwrap();
        assert_eq!(skills.get(&3300), Some(&5));
        assert_eq!(skills.get(&3301), None);
    }

    #[test]
    fn callback_query_is_read() {
        let request = "GET /callback?code=abc123&state=xyz HTTP/1.1\r\nHost: localhost\r\n\r\n";
        assert_eq!(query_param(request, "code").as_deref(), Some("abc123"));
        assert_eq!(query_param(request, "state").as_deref(), Some("xyz"));
    }
}
