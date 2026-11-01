use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Int(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

impl Value {
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Int(v) => Some(*v),
            Value::Real(v) => Some(*v as i64),
            Value::Text(s) => s.parse().ok(),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Int(v) => Some(*v as f64),
            Value::Real(v) => Some(*v),
            Value::Text(s) => s.parse().ok(),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Text(s) => Some(s),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct Table {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
}

impl Table {
    pub fn index(&self, column: &str) -> Option<usize> {
        self.columns
            .iter()
            .position(|c| c.eq_ignore_ascii_case(column))
    }

    pub fn get<'a>(&self, row: &'a [Value], column: &str) -> &'a Value {
        self.index(column)
            .and_then(|i| row.get(i))
            .unwrap_or(&Value::Null)
    }
}

pub struct Database {
    bytes: Vec<u8>,
    page_size: usize,
    reserved: usize,
    schema: HashMap<String, (u32, Vec<String>, Option<usize>)>,
}

#[derive(Debug)]
pub enum SqliteError {
    Io(std::io::Error),
    NotSqlite,
    Corrupt(&'static str),
    NoTable(String),
}

impl std::fmt::Display for SqliteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SqliteError::Io(e) => write!(f, "{e}"),
            SqliteError::NotSqlite => write!(f, "not an SQLite file"),
            SqliteError::Corrupt(what) => write!(f, "damaged SQLite file: {what}"),
            SqliteError::NoTable(t) => write!(f, "no table {t}"),
        }
    }
}

fn varint(bytes: &[u8], mut pos: usize) -> Option<(i64, usize)> {
    let mut value: i64 = 0;
    for i in 0..9 {
        let b = *bytes.get(pos)?;
        pos += 1;
        if i == 8 {
            value = (value << 8) | b as i64;
            return Some((value, pos));
        }
        value = (value << 7) | (b & 0x7f) as i64;
        if b & 0x80 == 0 {
            return Some((value, pos));
        }
    }
    None
}

fn be_u16(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes([*b.get(at)?, *b.get(at + 1)?]))
}

fn be_u32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes([
        *b.get(at)?,
        *b.get(at + 1)?,
        *b.get(at + 2)?,
        *b.get(at + 3)?,
    ]))
}

impl Database {
    pub fn open(path: &Path) -> Result<Database, SqliteError> {
        let bytes = std::fs::read(path).map_err(SqliteError::Io)?;
        Database::from_bytes(bytes)
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Result<Database, SqliteError> {
        if bytes.len() < 100 || &bytes[..16] != b"SQLite format 3\0" {
            return Err(SqliteError::NotSqlite);
        }
        let raw = be_u16(&bytes, 16).ok_or(SqliteError::Corrupt("header"))? as usize;
        let page_size = if raw == 1 { 65536 } else { raw };
        let reserved = bytes[20] as usize;
        let mut db = Database {
            bytes,
            page_size,
            reserved,
            schema: HashMap::new(),
        };
        let master =
            db.read_table_at(1, &["type", "name", "tbl_name", "rootpage", "sql"], Some(0))?;
        for row in &master.rows {
            if master.get(row, "type").as_str() != Some("table") {
                continue;
            }
            let (Some(name), Some(root), Some(sql)) = (
                master.get(row, "name").as_str(),
                master.get(row, "rootpage").as_i64(),
                master.get(row, "sql").as_str(),
            ) else {
                continue;
            };
            let (columns, rowid_alias) = columns_of(sql);
            db.schema
                .insert(name.to_string(), (root as u32, columns, rowid_alias));
        }
        Ok(db)
    }

    pub fn has_table(&self, name: &str) -> bool {
        self.schema.contains_key(name)
    }

    pub fn table(&self, name: &str) -> Result<Table, SqliteError> {
        let (root, columns, rowid_alias) = self
            .schema
            .get(name)
            .ok_or_else(|| SqliteError::NoTable(name.to_string()))?;
        let names: Vec<&str> = columns.iter().map(String::as_str).collect();
        self.read_table_at(*root, &names, *rowid_alias)
    }

    fn page(&self, number: u32) -> Option<&[u8]> {
        let start = (number as usize - 1) * self.page_size;
        self.bytes.get(start..start + self.page_size)
    }

    fn usable(&self) -> usize {
        self.page_size - self.reserved
    }

    fn read_table_at(
        &self,
        root: u32,
        columns: &[&str],
        rowid_alias: Option<usize>,
    ) -> Result<Table, SqliteError> {
        let mut rows = Vec::new();
        self.walk(root, &mut rows, 0, rowid_alias)?;
        Ok(Table {
            columns: columns.iter().map(|c| c.to_string()).collect(),
            rows,
        })
    }

    fn walk(
        &self,
        number: u32,
        rows: &mut Vec<Vec<Value>>,
        depth: usize,
        rowid_alias: Option<usize>,
    ) -> Result<(), SqliteError> {
        if depth > 64 {
            return Err(SqliteError::Corrupt("tree too deep"));
        }
        let page = self
            .page(number)
            .ok_or(SqliteError::Corrupt("page out of range"))?;
        let offset = if number == 1 { 100 } else { 0 };
        let kind = page[offset];
        let cells = be_u16(page, offset + 3).ok_or(SqliteError::Corrupt("cell count"))? as usize;
        match kind {
            0x0d => {
                let ptr_base = offset + 8;
                for i in 0..cells {
                    let at = be_u16(page, ptr_base + i * 2)
                        .ok_or(SqliteError::Corrupt("cell pointer"))?
                        as usize;
                    let (payload_len, at) =
                        varint(page, at).ok_or(SqliteError::Corrupt("payload length"))?;
                    let (rowid, at) = varint(page, at).ok_or(SqliteError::Corrupt("rowid"))?;
                    let payload = self.payload(page, at, payload_len as usize)?;
                    rows.push(decode_record(&payload, rowid, rowid_alias)?);
                }
            }
            0x05 => {
                let right = be_u32(page, offset + 8).ok_or(SqliteError::Corrupt("right child"))?;
                let ptr_base = offset + 12;
                for i in 0..cells {
                    let at = be_u16(page, ptr_base + i * 2)
                        .ok_or(SqliteError::Corrupt("cell pointer"))?
                        as usize;
                    let child = be_u32(page, at).ok_or(SqliteError::Corrupt("child"))?;
                    self.walk(child, rows, depth + 1, rowid_alias)?;
                }
                self.walk(right, rows, depth + 1, rowid_alias)?;
            }
            _ => return Err(SqliteError::Corrupt("unexpected page type")),
        }
        Ok(())
    }

    fn payload(&self, page: &[u8], at: usize, total: usize) -> Result<Vec<u8>, SqliteError> {
        let usable = self.usable();
        let max_local = usable - 35;
        if total <= max_local {
            return page
                .get(at..at + total)
                .map(|s| s.to_vec())
                .ok_or(SqliteError::Corrupt("payload"));
        }
        let min_local = ((usable - 12) * 32 / 255) - 23;
        let mut local = min_local + (total - min_local) % (usable - 4);
        if local > max_local {
            local = min_local;
        }
        let mut out = page
            .get(at..at + local)
            .ok_or(SqliteError::Corrupt("local payload"))?
            .to_vec();
        let mut next = be_u32(page, at + local).ok_or(SqliteError::Corrupt("overflow pointer"))?;
        while next != 0 && out.len() < total {
            let overflow = self
                .page(next)
                .ok_or(SqliteError::Corrupt("overflow page"))?;
            next = be_u32(overflow, 0).ok_or(SqliteError::Corrupt("overflow chain"))?;
            let take = (total - out.len()).min(usable - 4);
            out.extend_from_slice(&overflow[4..4 + take]);
        }
        Ok(out)
    }
}

fn decode_record(
    payload: &[u8],
    rowid: i64,
    rowid_alias: Option<usize>,
) -> Result<Vec<Value>, SqliteError> {
    let (header_len, mut pos) = varint(payload, 0).ok_or(SqliteError::Corrupt("record header"))?;
    let header_end = header_len as usize;
    let mut types = Vec::new();
    while pos < header_end {
        let (t, next) = varint(payload, pos).ok_or(SqliteError::Corrupt("serial type"))?;
        types.push(t);
        pos = next;
    }
    let mut body = header_end;
    let mut out = Vec::with_capacity(types.len());
    for (i, t) in types.iter().enumerate() {
        let (value, size) = match *t {
            0 => (
                if Some(i) == rowid_alias {
                    Value::Int(rowid)
                } else {
                    Value::Null
                },
                0,
            ),
            1 => (Value::Int(read_int(payload, body, 1)?), 1),
            2 => (Value::Int(read_int(payload, body, 2)?), 2),
            3 => (Value::Int(read_int(payload, body, 3)?), 3),
            4 => (Value::Int(read_int(payload, body, 4)?), 4),
            5 => (Value::Int(read_int(payload, body, 6)?), 6),
            6 => (Value::Int(read_int(payload, body, 8)?), 8),
            7 => {
                let bits = read_int(payload, body, 8)? as u64;
                (Value::Real(f64::from_bits(bits)), 8)
            }
            8 => (Value::Int(0), 0),
            9 => (Value::Int(1), 0),
            n if n >= 12 && n % 2 == 0 => {
                let len = ((n - 12) / 2) as usize;
                let slice = payload
                    .get(body..body + len)
                    .ok_or(SqliteError::Corrupt("blob"))?;
                (Value::Blob(slice.to_vec()), len)
            }
            n if n >= 13 => {
                let len = ((n - 13) / 2) as usize;
                let slice = payload
                    .get(body..body + len)
                    .ok_or(SqliteError::Corrupt("text"))?;
                (
                    Value::Text(String::from_utf8_lossy(slice).into_owned()),
                    len,
                )
            }
            _ => return Err(SqliteError::Corrupt("serial type")),
        };
        body += size;
        out.push(value);
    }
    Ok(out)
}

fn read_int(payload: &[u8], at: usize, size: usize) -> Result<i64, SqliteError> {
    let slice = payload
        .get(at..at + size)
        .ok_or(SqliteError::Corrupt("integer"))?;
    let mut value: i64 = if slice[0] & 0x80 != 0 { -1 } else { 0 };
    for b in slice {
        value = (value << 8) | *b as i64;
    }
    Ok(value)
}

fn columns_of(sql: &str) -> (Vec<String>, Option<usize>) {
    let Some(start) = sql.find('(') else {
        return (Vec::new(), None);
    };
    let Some(end) = sql.rfind(')') else {
        return (Vec::new(), None);
    };
    let body = &sql[start + 1..end];
    let mut definitions = Vec::new();
    let mut depth = 0;
    let mut current = String::new();
    for c in body.chars() {
        match c {
            '(' => {
                depth += 1;
                current.push(c);
            }
            ')' => {
                depth -= 1;
                current.push(c);
            }
            ',' if depth == 0 => {
                definitions.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(c),
        }
    }
    if !current.trim().is_empty() {
        definitions.push(current.trim().to_string());
    }
    let mut columns: Vec<(String, bool)> = Vec::new();
    let mut table_pk: Option<String> = None;
    for def in &definitions {
        let upper = def.to_ascii_uppercase();
        let first = upper.split_whitespace().next().unwrap_or("");
        if matches!(
            first,
            "PRIMARY" | "FOREIGN" | "UNIQUE" | "CHECK" | "CONSTRAINT"
        ) {
            if let Some(open) = upper.find("PRIMARY KEY") {
                let rest = &def[open + "PRIMARY KEY".len()..];
                if let (Some(a), Some(b)) = (rest.find('('), rest.find(')')) {
                    let inner = &rest[a + 1..b];
                    if !inner.contains(',') {
                        table_pk = Some(unquote(inner.trim()));
                    }
                }
            }
            continue;
        }
        let name = unquote(def.split_whitespace().next().unwrap_or(""));
        let is_integer = upper.split_whitespace().nth(1) == Some("INTEGER");
        let inline_pk = upper.contains("PRIMARY KEY");
        columns.push((name, is_integer && inline_pk));
    }
    let mut alias = columns.iter().position(|(_, pk)| *pk);
    if alias.is_none()
        && let Some(pk) = table_pk
    {
        alias = columns.iter().position(|(name, _)| {
            name.eq_ignore_ascii_case(&pk) && {
                definitions.iter().any(|d| {
                    unquote(d.split_whitespace().next().unwrap_or("")).eq_ignore_ascii_case(&pk)
                        && d.to_ascii_uppercase().split_whitespace().nth(1) == Some("INTEGER")
                })
            }
        });
    }
    (columns.into_iter().map(|(n, _)| n).collect(), alias)
}

fn unquote(s: &str) -> String {
    s.trim_matches(|c| c == '"' || c == '`' || c == '[' || c == ']' || c == '\'')
        .to_string()
}
