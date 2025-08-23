use std::collections::HashMap;

use axum::body::Bytes;

use axum::http::StatusCode;
use serde::ser::SerializeStruct;
use serde::Serialize;

pub struct AxumResponse {
    pub status: StatusCode,
    pub headers: HashMap<String, String>,
    pub body: Bytes,
}

impl Serialize for AxumResponse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("AxumResponse", 3)?;
        state.serialize_field("status", &self.status.as_u16())?;
        state.serialize_field("headers", &self.headers)?;
        state.serialize_field("body", &self.body)?;
        state.end()
    }
}
