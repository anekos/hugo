
use chrono::offset::Utc;
use chrono::DateTime;



#[derive(Debug)]
pub struct Entry {
    pub key: String,
    pub value: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub expired_at: Option<DateTime<Utc>>,
}
