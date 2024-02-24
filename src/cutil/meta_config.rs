use std::error::Error;
use std::num::ParseIntError;
use std::string::FromUtf16Error;

use axum::http::header::InvalidHeaderValue;
use axum_extra::extract::multipart::MultipartError;
use sea_orm::DbErr;
use serde_json::to_string;

use crate::cutil::meta::Meta;

impl<T> Into<Result<T, Meta>> for Meta {
  fn into(self) -> Result<T, Meta> {
    Err(self)
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
