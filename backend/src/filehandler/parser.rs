#[derive(Debug, Clone, PartialEq)]
pub struct CalderaProfile {
    pub profile: ProfileMeta,
    pub modlist: Vec<ModEntry>,
    pub conflicts: Vec<ConflictRule>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProfileMeta {
    pub name: String,
    pub created: String,
    pub modified: String,
    pub description: Option<String>,
    pub deployer: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub summary: Option<String>,
    pub source: String,
    pub source_url: Option<String>,
    pub nexus_mod_id: Option<f64>,
    pub nexus_file_id: Option<f64>,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub file_size: Option<f64>,
    pub file_count: Option<f64>,
    pub file_types: Vec<String>,
    pub user_notes: Option<String>,
    pub favorite: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConflictRule {
    pub id: String,
    pub file: String,
    pub winner: Option<String>,
    pub rule: String,
}

#[derive(Debug, Clone, PartialEq)]
enum Value {
    Str(String),
    Num(f64),
    Bool(bool),
    Ident(String),
    Array(Vec<Value>),
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Ident(String),
    Str(String),
    Num(f64),
    Bool(bool),
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Eq,
    Comma,
    Eof,
}

fn strip_comments(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    while i < chars.len() {
        let c = chars[i];
        if in_string {
            out.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        if c == '"' {
            in_string = true;
            out.push(c);
            i += 1;
            continue;
        }

        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        if c == '#' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        out.push(c);
        i += 1;
    }
    out
}

struct Lexer {
    chars: Vec<char>,
    i: usize,
}

impl Lexer {
    fn new(text: &str) -> Self {
        Self {
            chars: text.chars().collect(),
            i: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.i).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.i += 1;
        Some(c)
    }

    fn consume_ws(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.i += 1;
            } else {
                break;
            }
        }
    }

    fn read_while<F: Fn(char) -> bool>(&mut self, pred: F) -> String {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if !pred(c) {
                break;
            }
            s.push(c);
            self.i += 1;
        }
        s
    }

    fn read_string(&mut self) -> Result<String, String> {
        // opening quote already consumed
        let mut out = String::new();
        let mut escaped = false;
        while let Some(c) = self.bump() {
            if escaped {
                out.push(match c {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    '"' => '"',
                    '\\' => '\\',
                    x => x,
                });
                escaped = false;
                continue;
            }
            if c == '\\' {
                escaped = true;
                continue;
            }
            if c == '"' {
                return Ok(out);
            }
            out.push(c);
        }
        Err("unterminated string".to_string())
    }

    fn next_token(&mut self) -> Result<Token, String> {
        self.consume_ws();
        let Some(c) = self.bump() else {
            return Ok(Token::Eof);
        };
        match c {
            '{' => Ok(Token::LBrace),
            '}' => Ok(Token::RBrace),
            '[' => Ok(Token::LBracket),
            ']' => Ok(Token::RBracket),
            '=' => Ok(Token::Eq),
            ',' => Ok(Token::Comma),
            '"' => Ok(Token::Str(self.read_string()?)),
            '-' | '0'..='9' => {
                let mut n = String::new();
                n.push(c);
                n.push_str(&self.read_while(|x| x.is_ascii_digit() || x == '.'));
                let parsed = n
                    .parse::<f64>()
                    .map_err(|_| format!("invalid number literal: {}", n))?;
                Ok(Token::Num(parsed))
            }
            _ if c.is_ascii_alphabetic() || c == '_' => {
                let mut ident = String::new();
                ident.push(c);
                // Accept '-' in identifiers for runtime compatibility with file-stem mod ids
                // (e.g. "my-mod-123"), even though canonical serialization should prefer snake_case.
                ident.push_str(
                    &self.read_while(|x| x.is_ascii_alphanumeric() || x == '_' || x == '-'),
                );
                match ident.as_str() {
                    "true" => Ok(Token::Bool(true)),
                    "false" => Ok(Token::Bool(false)),
                    _ => Ok(Token::Ident(ident)),
                }
            }
            _ => Err(format!("unexpected character '{}'", c)),
        }
    }
}

struct Parser {
    tokens: Vec<Token>,
    i: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, i: 0 }
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.i).unwrap_or(&Token::Eof)
    }

    fn bump(&mut self) -> Token {
        let t = self.peek().clone();
        self.i += 1;
        t
    }

    fn expect_ident(&mut self) -> Result<String, String> {
        match self.bump() {
            Token::Ident(s) => Ok(s),
            other => Err(format!("expected identifier, got {:?}", other)),
        }
    }

    fn expect(&mut self, expected: Token) -> Result<(), String> {
        let got = self.bump();
        if got == expected {
            Ok(())
        } else {
            Err(format!("expected {:?}, got {:?}", expected, got))
        }
    }

    fn parse_value(&mut self) -> Result<Value, String> {
        match self.bump() {
            Token::Str(s) => Ok(Value::Str(s)),
            Token::Num(n) => Ok(Value::Num(n)),
            Token::Bool(b) => Ok(Value::Bool(b)),
            Token::Ident(i) => Ok(Value::Ident(i)),
            Token::LBracket => {
                let mut values = Vec::new();
                loop {
                    if matches!(self.peek(), Token::RBracket) {
                        self.bump();
                        break;
                    }
                    let v = self.parse_value()?;
                    values.push(v);
                    if matches!(self.peek(), Token::Comma) {
                        self.bump();
                    } else if matches!(self.peek(), Token::RBracket) {
                        self.bump();
                        break;
                    } else {
                        return Err("array expected ',' or ']'".to_string());
                    }
                }
                Ok(Value::Array(values))
            }
            other => Err(format!("expected value, got {:?}", other)),
        }
    }

    fn parse_field_map(&mut self) -> Result<Vec<(String, Value)>, String> {
        let mut fields = Vec::new();
        while !matches!(self.peek(), Token::RBrace | Token::Eof) {
            let key = self.expect_ident()?;
            self.expect(Token::Eq)?;
            let value = self.parse_value()?;
            fields.push((key, value));
        }
        self.expect(Token::RBrace)?;
        Ok(fields)
    }
}

fn value_as_string(v: Option<&Value>, ctx: &str, required: bool) -> Result<Option<String>, String> {
    match v {
        Some(Value::Str(s)) => Ok(Some(s.clone())),
        Some(Value::Ident(s)) => Ok(Some(s.clone())),
        Some(other) => Err(format!(
            "{} expected string/identifier, got {:?}",
            ctx, other
        )),
        None if required => Err(format!("{} missing required field", ctx)),
        None => Ok(None),
    }
}

fn value_as_num(v: Option<&Value>, ctx: &str) -> Result<Option<f64>, String> {
    match v {
        Some(Value::Num(n)) => Ok(Some(*n)),
        Some(other) => Err(format!("{} expected number, got {:?}", ctx, other)),
        None => Ok(None),
    }
}

fn value_as_bool(v: Option<&Value>, ctx: &str) -> Result<Option<bool>, String> {
    match v {
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(other) => Err(format!("{} expected bool, got {:?}", ctx, other)),
        None => Ok(None),
    }
}

fn value_as_string_array(v: Option<&Value>, ctx: &str) -> Result<Vec<String>, String> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let Value::Array(items) = v else {
        return Err(format!("{} expected array", ctx));
    };
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        match item {
            Value::Str(s) | Value::Ident(s) => out.push(s.clone()),
            Value::Num(n) => out.push(n.to_string()),
            Value::Bool(b) => out.push(b.to_string()),
            Value::Array(_) => return Err(format!("{} contains nested array", ctx)),
        }
    }
    Ok(out)
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn q(s: &str) -> String {
    format!("\"{}\"", esc(s))
}

fn format_num(n: f64) -> String {
    if n.fract() == 0.0 {
        format!("{}", n as i64)
    } else {
        format!("{}", n)
    }
}

fn write_str_array(out: &mut String, key: &str, vals: &[String], indent: &str) {
    if vals.is_empty() {
        return;
    }
    let rendered = vals.iter().map(|v| q(v)).collect::<Vec<_>>().join(", ");
    out.push_str(&format!("{}{} = [{}]\n", indent, key, rendered));
}

pub fn parse_profile(text: &str) -> Result<CalderaProfile, String> {
    let stripped = strip_comments(text);
    let mut lx = Lexer::new(&stripped);
    let mut tokens = Vec::new();
    loop {
        let t = lx.next_token()?;
        let done = matches!(t, Token::Eof);
        tokens.push(t);
        if done {
            break;
        }
    }
    let mut p = Parser::new(tokens);

    let mut profile_fields: Option<Vec<(String, Value)>> = None;
    let mut modlist_entries: Vec<(String, Vec<(String, Value)>)> = Vec::new();
    let mut conflict_entries: Vec<(String, Vec<(String, Value)>)> = Vec::new();
    let mut seen_top = Vec::new();

    while !matches!(p.peek(), Token::Eof) {
        let top = p.expect_ident()?;
        p.expect(Token::LBrace)?;
        seen_top.push(top.clone());
        match top.as_str() {
            "profile" => {
                if profile_fields.is_some() {
                    return Err("duplicate top-level block: profile".to_string());
                }
                profile_fields = Some(p.parse_field_map()?);
            }
            "modlist" => {
                while !matches!(p.peek(), Token::RBrace | Token::Eof) {
                    let id = p.expect_ident()?;
                    p.expect(Token::LBrace)?;
                    let fields = p.parse_field_map()?;
                    modlist_entries.push((id, fields));
                }
                p.expect(Token::RBrace)?;
            }
            "conflicts" => {
                while !matches!(p.peek(), Token::RBrace | Token::Eof) {
                    let id = p.expect_ident()?;
                    p.expect(Token::LBrace)?;
                    let fields = p.parse_field_map()?;
                    conflict_entries.push((id, fields));
                }
                p.expect(Token::RBrace)?;
            }
            _ => return Err(format!("unknown top-level block: {}", top)),
        }
    }

    let mut top = seen_top;
    top.sort();
    top.dedup();
    if !top.contains(&"profile".to_string())
        || !top.contains(&"modlist".to_string())
        || !top.contains(&"conflicts".to_string())
    {
        return Err(
            "profile file must contain top-level blocks: profile, modlist, conflicts".to_string(),
        );
    }

    let pf = profile_fields.ok_or_else(|| "missing top-level profile block".to_string())?;
    let get_pf = |k: &str| pf.iter().find(|(kk, _)| kk == k).map(|(_, v)| v);
    let profile = ProfileMeta {
        name: value_as_string(get_pf("name"), "profile.name", true)?
            .ok_or_else(|| "profile.name missing required field".to_string())?,
        created: value_as_string(get_pf("created"), "profile.created", true)?
            .ok_or_else(|| "profile.created missing required field".to_string())?,
        modified: value_as_string(get_pf("modified"), "profile.modified", true)?
            .ok_or_else(|| "profile.modified missing required field".to_string())?,
        description: value_as_string(get_pf("description"), "profile.description", false)?,
        deployer: value_as_string(get_pf("deployer"), "profile.deployer", true)?
            .ok_or_else(|| "profile.deployer missing required field".to_string())?,
    };

    let mut modlist = Vec::with_capacity(modlist_entries.len());
    for (id, fields) in modlist_entries {
        let get = |k: &str| fields.iter().find(|(kk, _)| kk == k).map(|(_, v)| v);
        let source = value_as_string(get("source"), &format!("modlist.{}.source", id), true)?
            .ok_or_else(|| format!("modlist.{}.source missing required field", id))?;
        if !["nexus", "gamebanana", "moddb", "itch", "local"].contains(&source.as_str()) {
            return Err(format!("modlist.{}.source invalid value '{}'", id, source));
        }
        modlist.push(ModEntry {
            id: id.clone(),
            name: value_as_string(get("name"), &format!("modlist.{}.name", id), true)?
                .ok_or_else(|| format!("modlist.{}.name missing required field", id))?,
            version: value_as_string(get("version"), &format!("modlist.{}.version", id), true)?
                .ok_or_else(|| format!("modlist.{}.version missing required field", id))?,
            author: value_as_string(get("author"), &format!("modlist.{}.author", id), false)?,
            description: value_as_string(
                get("description"),
                &format!("modlist.{}.description", id),
                false,
            )?,
            summary: value_as_string(get("summary"), &format!("modlist.{}.summary", id), false)?,
            source,
            source_url: value_as_string(
                get("source_url"),
                &format!("modlist.{}.source_url", id),
                false,
            )?,
            nexus_mod_id: value_as_num(
                get("nexus_mod_id"),
                &format!("modlist.{}.nexus_mod_id", id),
            )?,
            nexus_file_id: value_as_num(
                get("nexus_file_id"),
                &format!("modlist.{}.nexus_file_id", id),
            )?,
            categories: value_as_string_array(
                get("categories"),
                &format!("modlist.{}.categories", id),
            )?,
            tags: value_as_string_array(get("tags"), &format!("modlist.{}.tags", id))?,
            file_size: value_as_num(get("file_size"), &format!("modlist.{}.file_size", id))?,
            file_count: value_as_num(get("file_count"), &format!("modlist.{}.file_count", id))?,
            file_types: value_as_string_array(
                get("file_types"),
                &format!("modlist.{}.file_types", id),
            )?,
            user_notes: value_as_string(
                get("user_notes"),
                &format!("modlist.{}.user_notes", id),
                false,
            )?,
            favorite: value_as_bool(get("favorite"), &format!("modlist.{}.favorite", id))?,
        });
    }

    let mut conflicts = Vec::with_capacity(conflict_entries.len());
    for (id, fields) in conflict_entries {
        let get = |k: &str| fields.iter().find(|(kk, _)| kk == k).map(|(_, v)| v);
        let rule = value_as_string(get("rule"), &format!("conflicts.{}.rule", id), true)?
            .ok_or_else(|| format!("conflicts.{}.rule missing required field", id))?;
        if !["use", "keep_both", "skip"].contains(&rule.as_str()) {
            return Err(format!("conflicts.{}.rule invalid value '{}'", id, rule));
        }
        let winner = value_as_string(get("winner"), &format!("conflicts.{}.winner", id), false)?;
        if rule == "use" && winner.is_none() {
            return Err(format!(
                "conflicts.{}.winner required when rule = \"use\"",
                id
            ));
        }
        conflicts.push(ConflictRule {
            id: id.clone(),
            file: value_as_string(get("file"), &format!("conflicts.{}.file", id), true)?
                .ok_or_else(|| format!("conflicts.{}.file missing required field", id))?,
            winner,
            rule,
        });
    }

    Ok(CalderaProfile {
        profile,
        modlist,
        conflicts,
    })
}

pub fn serialize_profile(profile: &CalderaProfile) -> String {
    let mut out = String::new();

    out.push_str("profile {\n");
    out.push_str(&format!("    name = {}\n", q(&profile.profile.name)));
    out.push_str(&format!("    created = {}\n", q(&profile.profile.created)));
    out.push_str(&format!(
        "    modified = {}\n",
        q(&profile.profile.modified)
    ));
    if let Some(description) = &profile.profile.description {
        out.push_str(&format!("    description = {}\n", q(description)));
    }
    out.push_str(&format!(
        "    deployer = {}\n",
        q(&profile.profile.deployer)
    ));
    out.push_str("}\n\n");

    out.push_str("modlist {\n");
    for entry in &profile.modlist {
        out.push_str(&format!("    {} {{\n", entry.id));
        out.push_str(&format!("        name = {}\n", q(&entry.name)));
        out.push_str(&format!("        version = {}\n", q(&entry.version)));
        if let Some(v) = &entry.author {
            out.push_str(&format!("        author = {}\n", q(v)));
        }
        if let Some(v) = &entry.description {
            out.push_str(&format!("        description = {}\n", q(v)));
        }
        if let Some(v) = &entry.summary {
            out.push_str(&format!("        summary = {}\n", q(v)));
        }
        out.push_str(&format!("        source = {}\n", q(&entry.source)));
        if let Some(v) = &entry.source_url {
            out.push_str(&format!("        source_url = {}\n", q(v)));
        }
        if let Some(v) = entry.nexus_mod_id {
            out.push_str(&format!("        nexus_mod_id = {}\n", format_num(v)));
        }
        if let Some(v) = entry.nexus_file_id {
            out.push_str(&format!("        nexus_file_id = {}\n", format_num(v)));
        }
        write_str_array(&mut out, "categories", &entry.categories, "        ");
        write_str_array(&mut out, "tags", &entry.tags, "        ");
        if let Some(v) = entry.file_size {
            out.push_str(&format!("        file_size = {}\n", format_num(v)));
        }
        if let Some(v) = entry.file_count {
            out.push_str(&format!("        file_count = {}\n", format_num(v)));
        }
        write_str_array(&mut out, "file_types", &entry.file_types, "        ");
        if let Some(v) = &entry.user_notes {
            out.push_str(&format!("        user_notes = {}\n", q(v)));
        }
        if let Some(v) = entry.favorite {
            out.push_str(&format!(
                "        favorite = {}\n",
                if v { "true" } else { "false" }
            ));
        }
        out.push_str("    }\n");
        out.push('\n');
    }
    out.push_str("}\n\n");

    out.push_str("conflicts {\n");
    for c in &profile.conflicts {
        out.push_str(&format!("    {} {{\n", c.id));
        out.push_str(&format!("        file = {}\n", q(&c.file)));
        if let Some(winner) = &c.winner {
            out.push_str(&format!("        winner = {}\n", winner));
        }
        out.push_str(&format!("        rule = {}\n", q(&c.rule)));
        out.push_str("    }\n");
        out.push('\n');
    }
    out.push_str("}\n");
    out
}

#[derive(Debug, Clone, PartialEq)]
pub struct SettingsSchema {
    pub groups: Vec<SettingsGroup>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SettingsGroup {
    pub id: String,
    pub label: String,
    pub entries: Vec<SettingsEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SettingsEntry {
    pub id: String,
    pub label: String,
    pub type_expr: String,
    pub default: String,
    pub hot: bool,
    pub desc: Option<String>,
}

fn skip_settings_ws(source: &str, idx: usize) -> usize {
    let mut i = idx;
    while i < source.len() {
        let Some(ch) = source[i..].chars().next() else {
            break;
        };
        if !ch.is_whitespace() {
            break;
        }
        i += ch.len_utf8();
    }
    i
}

fn read_settings_ident(source: &str, idx: usize) -> Result<(String, usize), String> {
    let mut i = skip_settings_ws(source, idx);
    let start = i;
    while i < source.len() {
        let Some(ch) = source[i..].chars().next() else {
            break;
        };
        if !(ch.is_ascii_alphanumeric() || ch == '_') {
            break;
        }
        i += ch.len_utf8();
    }
    if i == start {
        return Err(format!("expected identifier at index {}", i));
    }
    Ok((source[start..i].to_string(), i))
}

fn read_settings_quoted(source: &str, idx: usize) -> Result<(String, usize), String> {
    let mut i = skip_settings_ws(source, idx);
    if source[i..].chars().next() != Some('"') {
        return Err(format!("expected quoted string at index {}", i));
    }
    i += 1;
    let mut out = String::new();
    let mut escaped = false;
    while i < source.len() {
        let Some(ch) = source[i..].chars().next() else {
            break;
        };
        i += ch.len_utf8();
        if escaped {
            out.push(match ch {
                'n' => '\n',
                't' => '\t',
                'r' => '\r',
                '"' => '"',
                '\\' => '\\',
                x => x,
            });
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '"' {
            return Ok((out, i));
        }
        out.push(ch);
    }
    Err("unterminated quoted string".to_string())
}

fn extract_settings_block(source: &str, open_brace_idx: usize) -> Result<(&str, usize), String> {
    let mut depth = 0usize;
    let mut i = open_brace_idx;
    let mut in_string = false;
    let mut escaped = false;
    while i < source.len() {
        let Some(ch) = source[i..].chars().next() else {
            break;
        };
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            i += ch.len_utf8();
            continue;
        }
        if ch == '"' {
            in_string = true;
        } else if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Ok((&source[open_brace_idx + 1..i], i + ch.len_utf8()));
            }
        }
        i += ch.len_utf8();
    }
    Err("unterminated block".to_string())
}

fn read_settings_field_value(body: &str, idx: usize) -> Result<(String, usize), String> {
    let mut i = skip_settings_ws(body, idx);
    if body[i..].chars().next() != Some('=') {
        return Err(format!("expected '=' at index {}", i));
    }
    i += 1;
    i = skip_settings_ws(body, i);
    let start = i;
    let mut in_string = false;
    let mut escaped = false;
    while i < body.len() {
        let Some(ch) = body[i..].chars().next() else {
            break;
        };
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            i += ch.len_utf8();
            continue;
        }
        if ch == '"' {
            in_string = true;
        } else if ch == '\n' || ch == '\r' {
            break;
        }
        i += ch.len_utf8();
    }
    Ok((body[start..i].trim().to_string(), i))
}

fn unquote_settings_value(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

fn validate_settings_type(type_expr: &str, entry_id: &str) -> Result<(), String> {
    let t = type_expr.trim();
    if ["bool", "select", "text", "path", "color", "keybind"].contains(&t) {
        return Ok(());
    }
    if t.starts_with("cycle [") && t.ends_with(']') {
        return Ok(());
    }
    if t.starts_with("range ") && t.contains("..") && t.contains(" step ") {
        return Ok(());
    }
    Err(format!("unknown type expression for {}: {}", entry_id, t))
}

fn parse_settings_fields(entry_id: String, body: &str) -> Result<SettingsEntry, String> {
    let mut label = None;
    let mut type_expr = None;
    let mut default = None;
    let mut hot = None;
    let mut desc = None;
    let mut idx = 0usize;
    while idx < body.len() {
        idx = skip_settings_ws(body, idx);
        if idx >= body.len() {
            break;
        }
        let (field, next) = read_settings_ident(body, idx)?;
        let (value, next) = read_settings_field_value(body, next)?;
        match field.as_str() {
            "label" => label = Some(unquote_settings_value(&value)),
            "type" => type_expr = Some(value),
            "default" => default = Some(unquote_settings_value(&value)),
            "hot" => {
                hot = Some(match value.as_str() {
                    "true" => true,
                    "false" => false,
                    _ => return Err(format!("{}.hot must be true or false", entry_id)),
                })
            }
            "desc" => desc = Some(unquote_settings_value(&value)),
            _ => {
                return Err(format!(
                    "unknown field {} in settings entry {}",
                    field, entry_id
                ))
            }
        }
        idx = next;
    }

    let type_expr = type_expr.ok_or_else(|| format!("{}.type missing required field", entry_id))?;
    validate_settings_type(&type_expr, &entry_id)?;
    Ok(SettingsEntry {
        id: entry_id.clone(),
        label: label.ok_or_else(|| format!("{}.label missing required field", entry_id))?,
        type_expr,
        default: default.ok_or_else(|| format!("{}.default missing required field", entry_id))?,
        hot: hot.ok_or_else(|| format!("{}.hot missing required field", entry_id))?,
        desc,
    })
}

fn parse_settings_entries(group_body: &str) -> Result<Vec<SettingsEntry>, String> {
    let mut entries = Vec::new();
    let mut idx = 0usize;
    while idx < group_body.len() {
        idx = skip_settings_ws(group_body, idx);
        if idx >= group_body.len() {
            break;
        }
        let (entry_id, next) = read_settings_ident(group_body, idx)?;
        let cursor = skip_settings_ws(group_body, next);
        if group_body[cursor..].chars().next() != Some('{') {
            return Err(format!("expected '{{' after settings entry {}", entry_id));
        }
        let (entry_body, next) = extract_settings_block(group_body, cursor)?;
        entries.push(parse_settings_fields(entry_id, entry_body)?);
        idx = next;
    }
    Ok(entries)
}

pub fn parse_settings_schema(text: &str) -> Result<SettingsSchema, String> {
    let source = strip_comments(text);
    let mut groups = Vec::new();
    let mut idx = 0usize;
    while idx < source.len() {
        idx = skip_settings_ws(&source, idx);
        if idx >= source.len() {
            break;
        }
        let (group_id, next) = read_settings_ident(&source, idx)?;
        let (group_label, next) = read_settings_quoted(&source, next)?;
        let cursor = skip_settings_ws(&source, next);
        if source[cursor..].chars().next() != Some('{') {
            return Err(format!("expected '{{' after settings group {}", group_id));
        }
        let (group_body, next) = extract_settings_block(&source, cursor)?;
        groups.push(SettingsGroup {
            id: group_id,
            label: group_label,
            entries: parse_settings_entries(group_body)?,
        });
        idx = next;
    }
    Ok(SettingsSchema { groups })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FULL_EXAMPLE: &str = r#"
// PROFILE — Combat Overhaul
profile {
    name        = "Combat Overhaul"
    created     = "2025-12-25T14:32:00Z"
    modified    = "2025-12-26T09:15:00Z"
    description = "Harder enemies + better weapons"
    deployer    = "unreal_engine"
}

modlist {
    mod_alpha_001 {
        name          = "Adds Gun Dildos Into The Game"
        version       = "1.2.0"
        author        = "modder_name"
        description   = "Long description here"
        summary       = "Short desc"
        source        = "nexus"          # nexus | gamebanana | moddb | itch | local
        source_url    = "https://..."
        nexus_mod_id  = 12345
        nexus_file_id = 67890
        categories = ["weapons"]
        tags       = ["funny", "weapons"]
        file_size  = 4523123
        file_count = 3
        file_types = [".pak", ".utoc", ".ucas"]
        user_notes = "this one breaks if loaded after X"
        favorite   = false
    }
    mod_beta_002 {
        name = "Beta"
        version = "0.1"
        source = "local"
    }
}

conflicts {
    weapons_pak_clash {
        file   = "Content/Paks/~mods/Weapons_P.pak"
        winner = mod_alpha_001
        rule   = "use"
    }
    combat_pak_clash {
        file = "Content/Paks/~mods/Combat_P.pak"
        rule = "keep_both"
    }
}
"#;

    #[test]
    fn parses_full_example() {
        let p = parse_profile(FULL_EXAMPLE).expect("parse should succeed");
        assert_eq!(p.profile.name, "Combat Overhaul");
        assert_eq!(p.modlist.len(), 2);
        assert_eq!(p.modlist[0].id, "mod_alpha_001");
        assert_eq!(p.modlist[1].id, "mod_beta_002");
        assert_eq!(p.conflicts.len(), 2);
    }

    #[test]
    fn strips_comments_correctly() {
        let t = r#"
profile {
    name = "x // not comment"
    created = "2025"
    modified = "2026"
    deployer = "unreal_engine" # inline
}
modlist { m { name="n" version="1" source="local" } }
conflicts {}
"#;
        let p = parse_profile(t).expect("parse should succeed");
        assert_eq!(p.profile.name, "x // not comment");
    }

    #[test]
    fn validates_conflict_rule_and_winner() {
        let t = r#"
profile { name="x" created="c" modified="m" deployer="d" }
modlist {}
conflicts { c { file="a" rule="use" } }
"#;
        let err = parse_profile(t).expect_err("should fail");
        assert!(err.contains("winner required"));
    }

    #[test]
    fn rejects_missing_profile_fields() {
        let t = r#"
profile { name="x" created="c" deployer="d" }
modlist {}
conflicts {}
"#;
        let err = parse_profile(t).expect_err("should fail");
        assert!(err.contains("profile.modified"));
    }

    #[test]
    fn rejects_invalid_source() {
        let t = r#"
profile { name="x" created="c" modified="m" deployer="d" }
modlist { m { name="n" version="1" source="steam" } }
conflicts {}
"#;
        let err = parse_profile(t).expect_err("should fail");
        assert!(err.contains("source invalid value"));
    }

    #[test]
    fn serializes_canonical_and_roundtrips() {
        let p = parse_profile(FULL_EXAMPLE).expect("parse should succeed");
        let text = serialize_profile(&p);
        assert!(text.starts_with("profile {"));
        assert!(text.contains("modlist {"));
        assert!(text.contains("conflicts {"));
        let p2 = parse_profile(&text).expect("roundtrip parse should succeed");
        assert_eq!(p, p2);
    }

    #[test]
    fn minimal_roundtrip() {
        let t = r#"
profile { name="x" created="c" modified="m" deployer="d" }
modlist {}
conflicts {}
"#;
        let p = parse_profile(t).expect("parse should succeed");
        let p2 = parse_profile(&serialize_profile(&p)).expect("roundtrip parse should succeed");
        assert_eq!(p, p2);
    }
}
