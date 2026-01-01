use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;
use std::fmt;
use utoipa::ToSchema;

#[macro_export]
macro_rules! meta {
  ($name: expr) => {
    $crate::cutil::meta::Meta {
      name: $name.to_string(),
      message: "".to_string(),
      data: None,
    }
    .into()
  };
  ($name: expr, $message: expr) => {
    $crate::cutil::meta::Meta {
      name: $name.to_string(),
      message: $message.to_string(),
      data: None,
    }
    .into()
  };
  ($name: expr, $message: expr, $data: expr) => {
    $crate::cutil::meta::Meta {
      name: $name.to_string(),
      message: $message.to_string(),
      data: Some($data),
    }
    .into()
  };
}

pub type R<T> = std::result::Result<T, Meta>;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Meta {
  pub name: String,
  pub message: String,
  pub data: Option<Value>,
}

unsafe impl Send for Meta {}

unsafe impl Sync for Meta {}

impl Error for Meta {}

impl fmt::Display for Meta {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "Meta: name={}, message={}, data: {:?}", self.name, self.message, self.data)
  }
}

// =============================================================================
// Meta 转换实现
// =============================================================================

use std::num::ParseIntError;
use std::string::FromUtf16Error;

use axum::http::header::InvalidHeaderValue;
use axum_extra::extract::multipart::MultipartError;
use sea_orm::DbErr;
use serde_json::to_string;

impl<T> From<Meta> for Result<T, Meta> {
  fn from(meta: Meta) -> Self {
    Err(meta)
  }
}

impl axum::response::IntoResponse for Meta {
  fn into_response(self) -> axum::response::Response {
    let status_code = match self.name.as_str() {
      "unauthorized" => http::StatusCode::UNAUTHORIZED,
      _ => http::StatusCode::INTERNAL_SERVER_ERROR,
    };
    let text = to_string(&self).unwrap_or_default();
    (status_code, text).into_response()
  }
}

impl From<&'_ (dyn Error + 'static)> for Meta {
  fn from(error: &'_ (dyn Error + 'static)) -> Self {
    Meta {
      name: "error".to_string(),
      message: error.to_string(),
      data: None,
    }
  }
}

impl From<Box<dyn Error + 'static>> for Meta {
  fn from(error: Box<dyn Error + 'static>) -> Self {
    Meta::from(&*error)
  }
}

impl From<serde_json::Error> for Meta {
  fn from(error: serde_json::Error) -> Self {
    Meta {
      name: "serde_json_error".to_string(),
      message: error.to_string(),
      data: None,
    }
  }
}

impl From<ParseIntError> for Meta {
  fn from(error: ParseIntError) -> Self {
    Meta {
      name: "parse_int_error".to_string(),
      message: error.to_string(),
      data: None,
    }
  }
}

impl From<InvalidHeaderValue> for Meta {
  fn from(error: InvalidHeaderValue) -> Self {
    Meta {
      name: "invalid_header_value".to_string(),
      message: error.to_string(),
      data: None,
    }
  }
}

impl From<std::io::Error> for Meta {
  fn from(error: std::io::Error) -> Self {
    Meta {
      name: "io_error".to_string(),
      message: error.to_string(),
      data: None,
    }
  }
}

impl From<MultipartError> for Meta {
  fn from(error: MultipartError) -> Self {
    Meta {
      name: "multipart_error".to_string(),
      message: error.to_string(),
      data: None,
    }
  }
}

impl From<FromUtf16Error> for Meta {
  fn from(error: FromUtf16Error) -> Self {
    Meta {
      name: "from_utf16_error".to_string(),
      message: error.to_string(),
      data: None,
    }
  }
}

impl From<rumqttc::ClientError> for Meta {
  fn from(error: rumqttc::ClientError) -> Self {
    Meta {
      name: "rumqttc_client_error".to_string(),
      message: error.to_string(),
      data: None,
    }
  }
}

impl From<rumqttc::v5::ClientError> for Meta {
  fn from(error: rumqttc::v5::ClientError) -> Self {
    Meta {
      name: "rumqttc_client_error".to_string(),
      message: error.to_string(),
      data: None,
    }
  }
}

impl From<tokio::sync::watch::error::SendError<bool>> for Meta {
  fn from(error: tokio::sync::watch::error::SendError<bool>) -> Self {
    Meta {
      name: "sync_watch_send_error".to_string(),
      message: error.to_string(),
      data: None,
    }
  }
}

impl From<reqwest::Error> for Meta {
  fn from(error: reqwest::Error) -> Self {
    Meta {
      name: "reqwest_error".to_string(),
      message: error.to_string(),
      data: None,
    }
  }
}

impl From<DbErr> for Meta {
  fn from(error: DbErr) -> Self {
    Meta {
      name: "db_err".to_string(),
      message: error.to_string(),
      data: None,
    }
  }
}

impl From<(&str, &str)> for Meta {
  fn from(error: (&str, &str)) -> Self {
    Meta {
      name: error.0.to_string(),
      message: error.1.to_string(),
      data: None,
    }
  }
}

impl From<anyhow::Error> for Meta {
  fn from(error: anyhow::Error) -> Self {
    Meta {
      name: "anyhow_error".to_string(),
      message: error.to_string(),
      data: None,
    }
  }
}
