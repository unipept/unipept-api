use axum::{async_trait, extract::FromRequestParts, http::{request::Parts, StatusCode}};

pub mod pept2ec;
pub mod pept2funct;
pub mod pept2go;
pub mod pept2interpro;
pub mod pept2lca;
pub mod pept2prot;
pub mod pept2taxa;
pub mod peptinfo;
pub mod protinfo;
pub mod taxa2lca;
pub mod taxa2tree;
pub mod taxonomy;

pub fn default_equate_il() -> bool {
  true
}

pub fn default_extra() -> bool {
  false
}

pub fn default_domains() -> bool {
  false
}

pub fn default_names() -> bool {
  false
}

pub struct Query<T>(T);

#[async_trait]
impl<S, T> FromRequestParts<S> for Query<T>
where
  S: Send + Sync,
  T: serde::de::DeserializeOwned,
{
  type Rejection = (StatusCode, &'static str);

  async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
    let query = parts.uri.query().ok_or((StatusCode::BAD_REQUEST, "missing query string"))?;
    Ok(Self(serde_qs::from_str(query).map_err(|_| (StatusCode::BAD_REQUEST, "invalid query string"))?))
  }
}
