//! Custom fields (Project > Custom Fields): Text1-30, Number1-20, Flag1-20, Date1-10, Cost1-10,
//! Duration1-10, Start1-10 and Finish1-10 of tasks and resources, with a renamed title (alias),
//! a lookup list of values and a formula. Values live on the task / resource; formulas are worked
//! out after every recalculation.

use crate::model::{Project, Task};
use chrono::{Datelike, NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, PartialOrd, Ord)]
pub enum CfKind {
    Text,
    Number,
    Flag,
    Date,
    Cost,
    Duration,
    Start,
    Finish,
}

impl CfKind {
    pub const ALL: [CfKind; 8] = [CfKind::Cost, CfKind::Date, CfKind::Duration, CfKind::Finish, CfKind::Flag, CfKind::Number, CfKind::Start, CfKind::Text];
    pub fn label(self) -> &'static str {
        match self {
            CfKind::Text => "Text",
            CfKind::Number => "Number",
            CfKind::Flag => "Flag",
            CfKind::Date => "Date",
            CfKind::Cost => "Cost",
            CfKind::Duration => "Duration",
            CfKind::Start => "Start",
            CfKind::Finish => "Finish",
        }
    }
    pub fn count(self) -> u8 {
        match self {
            CfKind::Text => 30,
            CfKind::Number | CfKind::Flag => 20,
            _ => 10,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, PartialOrd, Ord)]
pub struct CustomField {
    pub kind: CfKind,
    /// 1-based
    pub n: u8,
}

impl CustomField {
    pub fn new(kind: CfKind, n: u8) -> Self {
        CustomField { kind, n }
    }

    /// The standard field name ("Text1").
    pub fn name(self) -> &'static str {
        static NAMES: OnceLock<std::collections::HashMap<CustomField, &'static str>> = OnceLock::new();
        NAMES.get_or_init(|| all().into_iter().map(|f| (f, &*Box::leak(format!("{}{}", f.kind.label(), f.n).into_boxed_str()))).collect()).get(&self).copied().unwrap_or("")
    }

    pub fn parse(s: &str) -> Option<CustomField> {
        all().into_iter().find(|f| f.name().eq_ignore_ascii_case(s.trim()))
    }

    /// The field number (XML FieldID) of the task or resource field.
    pub fn field_id(self, resource: bool) -> u32 {
        crate::custom_tables::spec(resource, self.kind, self.n).map(|s| s.var).filter(|v| *v != 0).unwrap_or_else(|| flag_id(resource, self.n))
    }
}

/// field numbers of Flag1-20 (no var item in a .mpp: the flag is a bit).
fn flag_id(resource: bool, n: u8) -> u32 {
    // field numbers: task Flag1-10 188743752..,
    // Flag11-20 188743972..; resource Flag1-9 205521023.., Flag10 205521022, Flag11-20 205521091..
    let n = n as u32;
    match (resource, n) {
        (false, 1..=10) => 188743752 + n - 1,
        (false, _) => 188743972 + n - 11,
        (true, 10) => 205521022,
        (true, 1..=9) => 205521023 + n - 1,
        (true, _) => 205521091 + n - 11,
    }
}

/// Every custom field, by kind.
pub fn all() -> Vec<CustomField> {
    let mut v = Vec::new();
    for k in [CfKind::Text, CfKind::Number, CfKind::Flag, CfKind::Date, CfKind::Cost, CfKind::Duration, CfKind::Start, CfKind::Finish] {
        for n in 1..=k.count() {
            v.push(CustomField::new(k, n));
        }
    }
    v
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CfValue {
    Text(String),
    Number(f64),
    Flag(bool),
    Date(NaiveDateTime),
    Cost(f64),
    /// working minutes
    Duration(i64),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CustomFieldDef {
    pub field: CustomField,
    /// false = task field, true = resource field
    #[serde(default)]
    pub resource: bool,
    #[serde(default)]
    pub alias: String,
    /// Lookup: (value, description)
    #[serde(default)]
    pub values: Vec<(String, String)>,
    /// Restrict field to items in the value list
    #[serde(default)]
    pub restrict: bool,
    /// Formula (the standard syntax: [Field] references, operators, IIf and the usual functions)
    #[serde(default)]
    pub formula: String,
}

impl CustomFieldDef {
    pub fn new(field: CustomField, resource: bool) -> Self {
        CustomFieldDef { field, resource, alias: String::new(), values: Vec::new(), restrict: false, formula: String::new() }
    }
}

/// Values of a task or resource: (field, value).
pub type Values = Vec<(CustomField, CfValue)>;

pub fn get(v: &Values, f: CustomField) -> Option<&CfValue> {
    v.iter().find(|x| x.0 == f).map(|x| &x.1)
}

pub fn set(v: &mut Values, f: CustomField, val: Option<CfValue>) {
    v.retain(|x| x.0 != f);
    if let Some(val) = val {
        v.push((f, val));
        v.sort_by(|a, b| a.0.cmp(&b.0));
    }
}

/// A value as MPP shows it.
pub fn format(p: &Project, f: CustomField, v: Option<&CfValue>, fmt_date: &dyn Fn(NaiveDate) -> String, fmt_dur: &dyn Fn(i64) -> String) -> String {
    match (f.kind, v) {
        (CfKind::Flag, Some(CfValue::Flag(b))) => if *b { "Yes".into() } else { "No".into() },
        (CfKind::Flag, _) => "No".into(),
        (CfKind::Number, Some(CfValue::Number(x))) => trim_num(*x),
        (CfKind::Number, _) => "0".into(),
        (CfKind::Cost, Some(CfValue::Cost(x))) => crate::util::fmt_money(*x),
        (CfKind::Cost, _) => crate::util::fmt_money(0.0),
        (CfKind::Duration, Some(CfValue::Duration(m))) => fmt_dur(*m),
        (CfKind::Duration, _) => fmt_dur(0),
        (CfKind::Date | CfKind::Start | CfKind::Finish, Some(CfValue::Date(d))) => fmt_date(d.date()),
        (CfKind::Date | CfKind::Start | CfKind::Finish, _) => "NA".into(),
        (CfKind::Text, Some(CfValue::Text(s))) => s.clone(),
        _ => {
            let _ = p;
            String::new()
        }
    }
}

fn trim_num(x: f64) -> String {
    if (x - x.round()).abs() < 1e-9 {
        format!("{}", x.round() as i64)
    } else {
        let s = format!("{x:.4}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

/// Text typed in a custom field's cell -> its value (None = cleared).
pub fn parse_value(f: CustomField, text: &str, parse_date: &dyn Fn(&str) -> Option<NaiveDate>, parse_dur: &dyn Fn(&str) -> Option<i64>, day_start: &dyn Fn(NaiveDate) -> NaiveDateTime) -> Result<Option<CfValue>, String> {
    let t = text.trim();
    match f.kind {
        CfKind::Text => Ok(if t.is_empty() { None } else { Some(CfValue::Text(t.to_string())) }),
        CfKind::Flag => {
            let l = t.to_lowercase();
            Ok(Some(CfValue::Flag(matches!(l.as_str(), "yes" | "y" | "true" | "1" | "ใช่"))))
        }
        CfKind::Number | CfKind::Cost => {
            if t.is_empty() {
                return Ok(None);
            }
            let clean: String = t.chars().filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-').collect();
            let x: f64 = clean.parse().map_err(|_| "The value you entered isn't a number.".to_string())?;
            Ok(Some(if f.kind == CfKind::Cost { CfValue::Cost(x) } else { CfValue::Number(x) }))
        }
        CfKind::Duration => {
            if t.is_empty() {
                return Ok(None);
            }
            parse_dur(t).map(|m| Some(CfValue::Duration(m))).ok_or_else(|| "The duration you entered isn't valid.".to_string())
        }
        CfKind::Date | CfKind::Start | CfKind::Finish => {
            if t.is_empty() || t.eq_ignore_ascii_case("NA") {
                return Ok(None);
            }
            parse_date(t).map(|d| Some(CfValue::Date(day_start(d)))).ok_or_else(|| "The date you entered isn't supported.".to_string())
        }
    }
}

// ================================================================ formulas

#[derive(Clone, Debug, PartialEq)]
enum V {
    Num(f64),
    Str(String),
    Bool(bool),
    Date(NaiveDateTime),
    Na,
}

impl V {
    fn num(&self) -> f64 {
        match self {
            V::Num(x) => *x,
            V::Bool(b) => {
                if *b {
                    -1.0
                } else {
                    0.0
                }
            }
            V::Str(s) => s.trim().parse().unwrap_or(0.0),
            V::Date(d) => (d.and_utc().timestamp() as f64) / 86400.0 + 25569.0,
            V::Na => 0.0,
        }
    }
    fn truth(&self) -> bool {
        match self {
            V::Bool(b) => *b,
            V::Str(s) => matches!(s.to_lowercase().as_str(), "yes" | "true"),
            _ => self.num() != 0.0,
        }
    }
    fn text(&self) -> String {
        match self {
            V::Num(x) => trim_num(*x),
            V::Str(s) => s.clone(),
            V::Bool(b) => if *b { "Yes".into() } else { "No".into() },
            V::Date(d) => d.format("%d/%m/%Y").to_string(),
            V::Na => "NA".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(f64),
    Str(String),
    Field(String),
    Ident(String),
    Op(String),
    LParen,
    RParen,
    Comma,
}

fn lex(s: &str) -> Result<Vec<Tok>, String> {
    let c: Vec<char> = s.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < c.len() {
        let ch = c[i];
        if ch.is_whitespace() {
            i += 1;
        } else if ch == '[' {
            let j = (i + 1..c.len()).find(|&j| c[j] == ']').ok_or("Missing ] in the formula.")?;
            out.push(Tok::Field(c[i + 1..j].iter().collect()));
            i = j + 1;
        } else if ch == '"' {
            let mut j = i + 1;
            let mut s = String::new();
            while j < c.len() {
                if c[j] == '"' {
                    if j + 1 < c.len() && c[j + 1] == '"' {
                        s.push('"');
                        j += 2;
                        continue;
                    }
                    break;
                }
                s.push(c[j]);
                j += 1;
            }
            out.push(Tok::Str(s));
            i = j + 1;
        } else if ch.is_ascii_digit() || (ch == '.' && i + 1 < c.len() && c[i + 1].is_ascii_digit()) {
            let j = (i..c.len()).find(|&j| !(c[j].is_ascii_digit() || c[j] == '.')).unwrap_or(c.len());
            out.push(Tok::Num(c[i..j].iter().collect::<String>().parse().map_err(|_| "Bad number in the formula.")?));
            i = j;
        } else if ch.is_alphabetic() || ch == '_' {
            let j = (i..c.len()).find(|&j| !(c[j].is_alphanumeric() || c[j] == '_')).unwrap_or(c.len());
            out.push(Tok::Ident(c[i..j].iter().collect()));
            i = j;
        } else if ch == '(' {
            out.push(Tok::LParen);
            i += 1;
        } else if ch == ')' {
            out.push(Tok::RParen);
            i += 1;
        } else if ch == ',' {
            out.push(Tok::Comma);
            i += 1;
        } else {
            let two: String = c[i..(i + 2).min(c.len())].iter().collect();
            if ["<>", "<=", ">="].contains(&two.as_str()) {
                out.push(Tok::Op(two));
                i += 2;
            } else if "+-*/&=<>^\\".contains(ch) {
                out.push(Tok::Op(ch.to_string()));
                i += 1;
            } else {
                return Err(format!("Unexpected '{ch}' in the formula."));
            }
        }
    }
    Ok(out)
}

struct Ctx<'a> {
    p: &'a Project,
    t: &'a Task,
}

impl Ctx<'_> {
    fn field(&self, name: &str) -> V {
        let t = self.t;
        let cal = &self.p.calendar;
        let n = name.trim().to_lowercase();
        let base = |k: u8| t.baselines.iter().find(|b| b.number == k);
        match n.as_str() {
            "name" => V::Str(t.name.clone()),
            "duration" => V::Num(t.dur_minutes(cal.minutes_per_day()) as f64),
            "work" => V::Num(t.work_min as f64),
            "cost" => V::Num(t.cost),
            "actual cost" => V::Num(t.actual_cost),
            "fixed cost" => V::Num(t.fixed_cost),
            "% complete" => V::Num(t.percent_complete as f64),
            "physical % complete" => V::Num(t.physical_percent as f64),
            "priority" => V::Num(t.priority as f64),
            "start" => t.start_dt.map(V::Date).unwrap_or(V::Na),
            "finish" => t.finish_dt.map(V::Date).unwrap_or(V::Na),
            "actual start" => t.actual_start.map(V::Date).unwrap_or(V::Na),
            "actual finish" => t.actual_finish.map(V::Date).unwrap_or(V::Na),
            "deadline" => t.deadline.map(|d| V::Date(cal.day_end(d))).unwrap_or(V::Na),
            "baseline start" => base(0).and_then(|b| b.start).map(V::Date).unwrap_or(V::Na),
            "baseline finish" => base(0).and_then(|b| b.finish).map(V::Date).unwrap_or(V::Na),
            "baseline duration" => V::Num(base(0).and_then(|b| b.dur_min).unwrap_or(0) as f64),
            "baseline cost" => V::Num(base(0).and_then(|b| b.cost).unwrap_or(0.0)),
            "baseline work" => V::Num(base(0).and_then(|b| b.work_min).unwrap_or(0) as f64),
            "total slack" => V::Num(t.slack_min as f64),
            "free slack" => V::Num(t.free_slack_min as f64),
            "critical" => V::Bool(t.critical),
            "milestone" => V::Bool(t.is_milestone()),
            "summary" => V::Bool(t.is_summary),
            "active" => V::Bool(t.active),
            "outline level" => V::Num(t.outline_level as f64),
            "unique id" => V::Num(t.uid as f64),
            "notes" => V::Str(t.notes.clone()),
            "wbs" => V::Str(t.wbs.clone()),
            "resource names" => V::Str(self.p.resource_names(t)),
            "status date" => self.p.status_date.map(|d| V::Date(cal.day_end(d))).unwrap_or(V::Na),
            "project start" => V::Date(cal.day_start(self.p.start)),
            _ => match CustomField::parse(name) {
                Some(f) => match get(&t.custom, f) {
                    Some(CfValue::Text(s)) => V::Str(s.clone()),
                    Some(CfValue::Number(x)) | Some(CfValue::Cost(x)) => V::Num(*x),
                    Some(CfValue::Flag(b)) => V::Bool(*b),
                    Some(CfValue::Date(d)) => V::Date(*d),
                    Some(CfValue::Duration(m)) => V::Num(*m as f64),
                    None => match f.kind {
                        CfKind::Text => V::Str(String::new()),
                        CfKind::Flag => V::Bool(false),
                        CfKind::Date | CfKind::Start | CfKind::Finish => V::Na,
                        _ => V::Num(0.0),
                    },
                },
                None => V::Na,
            },
        }
    }
}

struct Parser<'a> {
    toks: Vec<Tok>,
    pos: usize,
    cx: &'a Ctx<'a>,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }
    fn next(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.pos).cloned();
        self.pos += 1;
        t
    }
    fn ident_is(&self, w: &str) -> bool {
        matches!(self.peek(), Some(Tok::Ident(s)) if s.eq_ignore_ascii_case(w))
    }
    fn expr(&mut self) -> Result<V, String> {
        let mut a = self.and()?;
        while self.ident_is("or") {
            self.pos += 1;
            let b = self.and()?;
            a = V::Bool(a.truth() || b.truth());
        }
        Ok(a)
    }
    fn and(&mut self) -> Result<V, String> {
        let mut a = self.not()?;
        while self.ident_is("and") {
            self.pos += 1;
            let b = self.not()?;
            a = V::Bool(a.truth() && b.truth());
        }
        Ok(a)
    }
    fn not(&mut self) -> Result<V, String> {
        if self.ident_is("not") {
            self.pos += 1;
            let a = self.not()?;
            return Ok(V::Bool(!a.truth()));
        }
        self.cmp()
    }
    fn cmp(&mut self) -> Result<V, String> {
        let a = self.concat()?;
        if let Some(Tok::Op(o)) = self.peek().cloned() {
            if ["=", "<>", "<", ">", "<=", ">="].contains(&o.as_str()) {
                self.pos += 1;
                let b = self.concat()?;
                let ord = match (&a, &b) {
                    (V::Str(x), V::Str(y)) => x.to_lowercase().cmp(&y.to_lowercase()),
                    _ => a.num().partial_cmp(&b.num()).unwrap_or(std::cmp::Ordering::Equal),
                };
                use std::cmp::Ordering::*;
                return Ok(V::Bool(match o.as_str() {
                    "=" => ord == Equal,
                    "<>" => ord != Equal,
                    "<" => ord == Less,
                    ">" => ord == Greater,
                    "<=" => ord != Greater,
                    _ => ord != Less,
                }));
            }
        }
        Ok(a)
    }
    fn concat(&mut self) -> Result<V, String> {
        let mut a = self.add()?;
        while matches!(self.peek(), Some(Tok::Op(o)) if o == "&") {
            self.pos += 1;
            let b = self.add()?;
            a = V::Str(a.text() + &b.text());
        }
        Ok(a)
    }
    fn add(&mut self) -> Result<V, String> {
        let mut a = self.mul()?;
        loop {
            match self.peek() {
                Some(Tok::Op(o)) if o == "+" || o == "-" => {
                    let o = o.clone();
                    self.pos += 1;
                    let b = self.mul()?;
                    a = match (&a, &b, o.as_str()) {
                        (V::Str(x), V::Str(y), "+") => V::Str(format!("{x}{y}")),
                        (V::Date(d), _, "+") => V::Date(*d + chrono::Duration::seconds((b.num() * 86400.0) as i64)),
                        (V::Date(d), V::Date(e), "-") => V::Num((*d - *e).num_seconds() as f64 / 86400.0),
                        (V::Date(d), _, "-") => V::Date(*d - chrono::Duration::seconds((b.num() * 86400.0) as i64)),
                        (_, _, "+") => V::Num(a.num() + b.num()),
                        _ => V::Num(a.num() - b.num()),
                    };
                }
                _ => break,
            }
        }
        Ok(a)
    }
    fn mul(&mut self) -> Result<V, String> {
        let mut a = self.pow()?;
        loop {
            match self.peek() {
                Some(Tok::Op(o)) if o == "*" || o == "/" || o == "\\" => {
                    let o = o.clone();
                    self.pos += 1;
                    let b = self.pow()?;
                    a = match o.as_str() {
                        "*" => V::Num(a.num() * b.num()),
                        "\\" => V::Num(if b.num() == 0.0 { 0.0 } else { (a.num() / b.num()).trunc() }),
                        _ => {
                            if b.num() == 0.0 {
                                return Err("#ERROR: division by zero".into());
                            }
                            V::Num(a.num() / b.num())
                        }
                    };
                }
                Some(Tok::Ident(w)) if w.eq_ignore_ascii_case("mod") => {
                    self.pos += 1;
                    let b = self.pow()?;
                    a = V::Num(if b.num() == 0.0 { 0.0 } else { a.num() % b.num() });
                }
                _ => break,
            }
        }
        Ok(a)
    }
    fn pow(&mut self) -> Result<V, String> {
        let a = self.unary()?;
        if matches!(self.peek(), Some(Tok::Op(o)) if o == "^") {
            self.pos += 1;
            let b = self.unary()?;
            return Ok(V::Num(a.num().powf(b.num())));
        }
        Ok(a)
    }
    fn unary(&mut self) -> Result<V, String> {
        if matches!(self.peek(), Some(Tok::Op(o)) if o == "-") {
            self.pos += 1;
            let a = self.unary()?;
            return Ok(V::Num(-a.num()));
        }
        if matches!(self.peek(), Some(Tok::Op(o)) if o == "+") {
            self.pos += 1;
        }
        self.atom()
    }
    fn args(&mut self) -> Result<Vec<V>, String> {
        let mut out = Vec::new();
        if self.peek() != Some(&Tok::LParen) {
            return Ok(out);
        }
        self.pos += 1;
        if self.peek() == Some(&Tok::RParen) {
            self.pos += 1;
            return Ok(out);
        }
        loop {
            out.push(self.expr()?);
            match self.next() {
                Some(Tok::Comma) => continue,
                Some(Tok::RParen) => break,
                _ => return Err("Missing ) in the formula.".into()),
            }
        }
        Ok(out)
    }
    fn atom(&mut self) -> Result<V, String> {
        match self.next() {
            Some(Tok::Num(x)) => Ok(V::Num(x)),
            Some(Tok::Str(s)) => Ok(V::Str(s)),
            Some(Tok::Field(f)) => Ok(self.cx.field(&f)),
            Some(Tok::LParen) => {
                let v = self.expr()?;
                if self.next() != Some(Tok::RParen) {
                    return Err("Missing ) in the formula.".into());
                }
                Ok(v)
            }
            Some(Tok::Ident(name)) => {
                let lname = name.to_lowercase();
                match lname.as_str() {
                    "true" | "yes" => return Ok(V::Bool(true)),
                    "false" | "no" => return Ok(V::Bool(false)),
                    "na" => return Ok(V::Na),
                    _ => {}
                }
                let a = self.args()?;
                let cal = &self.cx.p.calendar;
                let n = |k: usize| a.get(k).map(|v| v.num()).unwrap_or(0.0);
                let s = |k: usize| a.get(k).map(|v| v.text()).unwrap_or_default();
                let d = |k: usize| match a.get(k) {
                    Some(V::Date(x)) => Some(*x),
                    _ => None,
                };
                Ok(match lname.as_str() {
                    "iif" => {
                        if a.first().map(|v| v.truth()).unwrap_or(false) {
                            a.get(1).cloned().unwrap_or(V::Na)
                        } else {
                            a.get(2).cloned().unwrap_or(V::Na)
                        }
                    }
                    "switch" => {
                        let mut r = V::Na;
                        for pair in a.chunks(2) {
                            if pair.len() == 2 && pair[0].truth() {
                                r = pair[1].clone();
                                break;
                            }
                        }
                        r
                    }
                    "choose" => a.get(n(0) as usize).cloned().unwrap_or(V::Na),
                    "abs" => V::Num(n(0).abs()),
                    "int" => V::Num(n(0).floor()),
                    "fix" => V::Num(n(0).trunc()),
                    "round" => {
                        let f = 10f64.powi(n(1) as i32);
                        V::Num((n(0) * f).round() / f)
                    }
                    "sqr" => V::Num(n(0).max(0.0).sqrt()),
                    "sgn" => V::Num(n(0).signum()),
                    "min" => V::Num(a.iter().map(|v| v.num()).fold(f64::INFINITY, f64::min)),
                    "max" => V::Num(a.iter().map(|v| v.num()).fold(f64::NEG_INFINITY, f64::max)),
                    "len" => V::Num(s(0).chars().count() as f64),
                    "left" => V::Str(s(0).chars().take(n(1) as usize).collect()),
                    "right" => {
                        let t = s(0);
                        let k = t.chars().count().saturating_sub(n(1) as usize);
                        V::Str(t.chars().skip(k).collect())
                    }
                    "mid" => {
                        let t = s(0);
                        let st = (n(1) as usize).max(1) - 1;
                        let len = if a.len() > 2 { n(2) as usize } else { usize::MAX };
                        V::Str(t.chars().skip(st).take(len).collect())
                    }
                    "ucase" => V::Str(s(0).to_uppercase()),
                    "lcase" => V::Str(s(0).to_lowercase()),
                    "trim" => V::Str(s(0).trim().to_string()),
                    "str" | "cstr" => V::Str(s(0)),
                    "val" | "cdbl" | "clng" | "cint" => V::Num(s(0).trim().parse::<f64>().unwrap_or(n(0))),
                    "instr" => V::Num(s(0).find(&s(1)).map(|i| s(0)[..i].chars().count() + 1).unwrap_or(0) as f64),
                    "now" => V::Date(chrono::Local::now().naive_local()),
                    "date" => V::Date(cal.day_start(chrono::Local::now().date_naive())),
                    "year" => V::Num(d(0).map(|x| x.year() as f64).unwrap_or(0.0)),
                    "month" => V::Num(d(0).map(|x| x.month() as f64).unwrap_or(0.0)),
                    "day" => V::Num(d(0).map(|x| x.day() as f64).unwrap_or(0.0)),
                    "weekday" => V::Num(d(0).map(|x| x.weekday().num_days_from_sunday() as f64 + 1.0).unwrap_or(0.0)),
                    "datediff" => {
                        let (Some(x), Some(y)) = (d(1), d(2)) else { return Ok(V::Na) };
                        let days = (y - x).num_seconds() as f64 / 86400.0;
                        V::Num(match s(0).to_lowercase().as_str() {
                            "ww" | "w" => (days / 7.0).trunc(),
                            "m" => ((y.year() - x.year()) * 12 + y.month() as i32 - x.month() as i32) as f64,
                            "yyyy" => (y.year() - x.year()) as f64,
                            "h" => (days * 24.0).trunc(),
                            "n" => (days * 1440.0).trunc(),
                            _ => days.trunc(),
                        })
                    }
                    "dateadd" => {
                        let Some(x) = d(2) else { return Ok(V::Na) };
                        let k = n(1);
                        V::Date(match s(0).to_lowercase().as_str() {
                            "ww" | "w" => x + chrono::Duration::days((k * 7.0) as i64),
                            "h" => x + chrono::Duration::minutes((k * 60.0) as i64),
                            "n" => x + chrono::Duration::minutes(k as i64),
                            _ => x + chrono::Duration::days(k as i64),
                        })
                    }
                    "projdatediff" => {
                        let (Some(x), Some(y)) = (d(0), d(1)) else { return Ok(V::Na) };
                        V::Num(cal.work_minutes(x, y) as f64)
                    }
                    "projdateadd" => {
                        let Some(x) = d(0) else { return Ok(V::Na) };
                        let m = n(1) as i64;
                        V::Date(if m >= 0 { cal.add_work_minutes(x, m) } else { cal.sub_work_minutes(x, -m) })
                    }
                    "projdatesub" => {
                        let Some(x) = d(0) else { return Ok(V::Na) };
                        V::Date(cal.sub_work_minutes(x, n(1) as i64))
                    }
                    "isnull" => V::Bool(matches!(a.first(), Some(V::Na)) || s(0).is_empty()),
                    _ => return Err(format!("Unknown function {name} in the formula.")),
                })
            }
            _ => Err("The formula is not complete.".into()),
        }
    }
}

/// Work out a formula for a task: the value for a field of `kind` (Err = #ERROR).
pub fn eval(p: &Project, t: &Task, formula: &str, kind: CfKind) -> Result<Option<CfValue>, String> {
    let toks = lex(formula)?;
    let cx = Ctx { p, t };
    let mut ps = Parser { toks, pos: 0, cx: &cx };
    let v = ps.expr()?;
    if ps.pos < ps.toks.len() {
        return Err("The formula has more than one expression.".into());
    }
    Ok(match (kind, v) {
        (_, V::Na) => None,
        (CfKind::Text, v) => Some(CfValue::Text(v.text())),
        (CfKind::Flag, v) => Some(CfValue::Flag(v.truth())),
        (CfKind::Number, v) => Some(CfValue::Number(v.num())),
        (CfKind::Cost, v) => Some(CfValue::Cost(v.num())),
        (CfKind::Duration, v) => Some(CfValue::Duration(v.num().round() as i64)),
        (CfKind::Date | CfKind::Start | CfKind::Finish, V::Date(d)) => Some(CfValue::Date(d)),
        (CfKind::Date | CfKind::Start | CfKind::Finish, _) => None,
    })
}

/// After a recalculation: every task field with a formula takes its result.
pub fn apply_formulas(p: &mut Project) {
    let defs: Vec<CustomFieldDef> = p.custom_defs.iter().filter(|d| !d.resource && !d.formula.trim().is_empty()).cloned().collect();
    if defs.is_empty() {
        return;
    }
    for i in 0..p.tasks.len() {
        for d in &defs {
            let v = eval(p, &p.tasks[i], &d.formula, d.field.kind).ok().flatten();
            set(&mut p.tasks[i].custom, d.field, v);
        }
    }
}

/// A task's / resource's custom fields as a .mpp row keeps them: var items read with `var`, flags
/// as FixedMeta bits (tables in custom_tables.rs).
pub fn read_mpp(resource: bool, var: &dyn Fn(u32) -> Option<Vec<u8>>, meta: &[u8]) -> Values {
    let mut out = Vec::new();
    for f in all() {
        let Some(s) = crate::custom_tables::spec(resource, f.kind, f.n) else { continue };
        let v = match f.kind {
            CfKind::Flag => {
                let (b, m) = s.bits[0];
                if meta.get(b as usize).map(|x| x & m != 0).unwrap_or(false) {
                    Some(CfValue::Flag(true))
                } else {
                    None
                }
            }
            CfKind::Text => var(s.var).map(|d| crate::mpp::utf16(&d)).filter(|t| !t.is_empty()).map(CfValue::Text),
            CfKind::Number => var(s.var).and_then(|d| crate::mpp::f64_at(&d, 0)).filter(|x| x.is_finite()).map(CfValue::Number),
            CfKind::Cost => var(s.var).and_then(|d| crate::mpp::f64_at(&d, 0)).filter(|x| x.is_finite()).map(|x| CfValue::Cost(x / 100.0)),
            CfKind::Duration => var(s.var).and_then(|d| crate::mpp::u32_at(&d, 0)).filter(|x| *x != u32::MAX).map(|x| CfValue::Duration(x as i64 / 10)),
            _ => var(s.var).and_then(|d| crate::mpp::datetime_at(&d, 0)).map(CfValue::Date),
        };
        if let Some(v) = v {
            out.push((f, v));
        }
    }
    out
}

/// The var item bytes of a value (None for flags, which are bits).
pub fn mpp_bytes(v: &CfValue) -> Option<Vec<u8>> {
    Some(match v {
        CfValue::Text(s) => crate::mpp::utf16z(s),
        CfValue::Number(x) => x.to_le_bytes().to_vec(),
        CfValue::Cost(x) => (x * 100.0).round().to_le_bytes().to_vec(),
        CfValue::Duration(m) => ((m * 10).clamp(0, u32::MAX as i64 - 1) as u32).to_le_bytes().to_vec(),
        CfValue::Date(d) => crate::mpp::timestamp(Some(*d)).to_le_bytes().to_vec(),
        CfValue::Flag(_) => return None,
    })
}

/// XML <Value> of a custom field.
pub fn xml_value(v: &CfValue) -> String {
    match v {
        CfValue::Text(s) => s.clone(),
        CfValue::Number(x) => trim_num(*x),
        CfValue::Flag(b) => (*b as u8).to_string(),
        CfValue::Date(d) => d.format("%Y-%m-%dT%H:%M:%S").to_string(),
        CfValue::Cost(x) => format!("{}", (x * 100.0).round() as i64),
        CfValue::Duration(m) => format!("PT{}H{}M0S", m / 60, m % 60),
    }
}

/// XML <Value> -> a value of the field.
pub fn from_xml(f: CustomField, s: &str) -> Option<CfValue> {
    let s = s.trim();
    Some(match f.kind {
        CfKind::Text => CfValue::Text(s.to_string()),
        CfKind::Number => CfValue::Number(s.parse().ok()?),
        CfKind::Flag => CfValue::Flag(s == "1" || s.eq_ignore_ascii_case("true")),
        CfKind::Cost => CfValue::Cost(s.parse::<f64>().ok()? / 100.0),
        CfKind::Duration => {
            let (h, rest) = s.trim_start_matches("PT").split_once('H')?;
            let (m, _) = rest.split_once('M')?;
            CfValue::Duration(h.parse::<f64>().ok()? as i64 * 60 + m.parse::<f64>().ok()? as i64)
        }
        _ => CfValue::Date(NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").ok()?),
    })
}

/// The field a field number (XML FieldID) stands for.
pub fn by_field_id(id: u32) -> Option<(CustomField, bool)> {
    for resource in [false, true] {
        for f in all() {
            if f.field_id(resource) == id {
                return Some((f, resource));
            }
        }
    }
    None
}

/// Renamed fields in a .mpp table's Props stream (TBkndTask/Props keeps, per renamed field,
/// its field number, the name's byte length and the UTF-16 name).
pub fn aliases_in(props: &[u8], resource: bool) -> Vec<(CustomField, String)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + 8 <= props.len() {
        let id = u32::from_le_bytes(props[i..i + 4].try_into().unwrap());
        if id >> 24 == if resource { 0x0C } else { 0x0B } {
            if let Some((f, r)) = by_field_id(id).filter(|x| x.1 == resource) {
                let n = u32::from_le_bytes(props[i + 4..i + 8].try_into().unwrap()) as usize;
                if n >= 2 && n <= 512 && n % 2 == 0 && i + 8 + n <= props.len() {
                    let name = crate::mpp::utf16(&props[i + 8..i + 8 + n]);
                    let printable = !name.is_empty() && name.chars().all(|c| !c.is_control());
                    if printable && !out.iter().any(|x: &(CustomField, String)| x.0 == f) {
                        let _ = r;
                        out.push((f, name));
                        i += 8 + n;
                        continue;
                    }
                }
            }
        }
        i += 2;
    }
    out
}

/// The definition of a field (alias, lookup, formula) when the project has one.
pub fn def(p: &Project, f: CustomField, resource: bool) -> Option<&CustomFieldDef> {
    p.custom_defs.iter().find(|d| d.field == f && d.resource == resource)
}

/// The title of a field: its alias, else "Text1".
pub fn title(p: &Project, f: CustomField, resource: bool) -> String {
    def(p, f, resource).map(|d| d.alias.trim().to_string()).filter(|a| !a.is_empty()).unwrap_or_else(|| f.name().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A recorded macro (ribbon commands) is kept as JSON and read back the same.
    #[test]
    fn macro_roundtrip() {
        use crate::ribbon::Cmd;
        let m: Vec<(String, Vec<Cmd>)> = vec![(
            "Macro1".into(),
            vec![Cmd::Indent, Cmd::Percent(50), Cmd::View(crate::app::View::TaskBoard), Cmd::Report(crate::reports::ReportKind::CashFlow), Cmd::InsertColumn(crate::app::TCol::Custom(CustomField::new(CfKind::Text, 3)))],
        )];
        let t = serde_json::to_string(&m).unwrap();
        let back: Vec<(String, Vec<Cmd>)> = serde_json::from_str(Box::leak(t.into_boxed_str())).unwrap();
        assert_eq!(format!("{back:?}"), format!("{m:?}"));
    }

    #[test]
    fn formulas() {
        let mut p = Project::new();
        let mut t = Task::new(1, "Pour concrete", p.start);
        t.set_duration_minutes(5 * 480, crate::model::DurUnit::Days, false, 480);
        t.percent_complete = 40;
        t.custom.push((CustomField::new(CfKind::Text, 1), CfValue::Text("Zone A".into())));
        p.tasks.push(t.clone());
        let ev = |f: &str, k: CfKind| eval(&p, &t, f, k).unwrap();
        assert_eq!(ev("[Duration]/480", CfKind::Number), Some(CfValue::Number(5.0)));
        assert_eq!(ev("IIf([% Complete]>=50, \"Late\", \"OK\")", CfKind::Text), Some(CfValue::Text("OK".into())));
        assert_eq!(ev("[Text1] & \" / \" & Left([Name], 4)", CfKind::Text), Some(CfValue::Text("Zone A / Pour".into())));
        assert_eq!(ev("[% Complete] > 10 And Not [Summary]", CfKind::Flag), Some(CfValue::Flag(true)));
        assert_eq!(ev("2 + 3 * 4 - 1", CfKind::Number), Some(CfValue::Number(13.0)));
        assert!(eval(&p, &t, "[Duration] / 0", CfKind::Number).is_err());
        assert_eq!(CustomField::parse("number12"), Some(CustomField::new(CfKind::Number, 12)));
        assert_eq!(CustomField::new(CfKind::Text, 1).field_id(false), 188743731);
    }
}
