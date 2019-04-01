
type Key = String;

pub enum Operation {
    Has(Key),
    Get(Key, Option<String>),
    Check(Key, Option<String>),
    Set(Key, Option<String>),
    Swap(Key, Option<String>),
    Modify(Key, Option<String>, bool),
    Import(String),
    Remove(Key),
    Shell(Vec<String>),
}

#[derive(Debug)]
pub struct Entry {
    pub key: String,
    pub value: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
