use serde_json::Value;
use volicord_types::schema::JsonObject;

use crate::pipeline::{CorePipelineError, CoreResult};

pub(crate) fn object_from_value(value: Value) -> CoreResult<JsonObject> {
    match value {
        Value::Object(object) => Ok(object),
        _ => Err(CorePipelineError::InvalidDispatch {
            detail: "expected JSON object".to_owned(),
        }),
    }
}
