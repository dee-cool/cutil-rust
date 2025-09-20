use crate::cutil::env::get_env_var_case_insensitive;
use crate::cutil::meta::R;
use tracing::info;

pub async fn create_reqwest_client() -> R<reqwest::Client> {
  let Some(http_proxy) = get_env_var_case_insensitive("http_proxy") else {
    let reqwest_client = reqwest::Client::new();
    return Ok(reqwest_client);
  };

  info!("create_client, using proxy: {}", http_proxy);
  let reqwest_proxy = reqwest::Proxy::all(http_proxy)?;
  let reqwest_client = reqwest::Client::builder().proxy(reqwest_proxy).build()?;

  Ok(reqwest_client)
}
