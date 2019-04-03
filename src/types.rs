
use chrono::offset::Utc;
use chrono::DateTime;



#[derive(Debug)]
pub struct Entry {
    pub key: String,
    pub value: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expired_at: Option<DateTime<Utc>>,
}
