use crate::utils::sign::SignError;
use std::convert::Infallible;
use thiserror::Error;

/// Instrument Errors.
#[derive(Debug, Error)]
pub enum InstrumentError {
    /// Instrument does not exist.
    #[error("instrument does not exist")]
    NotFound,
}

/// Rest API Errors.
#[derive(Debug, Error)]
pub enum RestError {
    /// API error message.
    #[error("api: code={0} msg={0}")]
    Api(i64, String),
    /// Http errors.
    #[error("http: {0}")]
    Http(#[from] http::Error),
    /// Errors from hyper.
    #[error("hyper: {0}")]
    Hyper(#[from] hyper::Error),
    /// Json errors.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    /// Urlencoded.
    #[error("urlencoded: {0}")]
    Urlencoded(#[from] serde_urlencoded::ser::Error),
    /// Standard exchange errors.
    #[error("exchange: {0}")]
    Exchange(#[from] ExchangeError),
    /// Invalid header value.
    #[error("invalid header value: {0}")]
    InvalidHeaderValue(#[from] http::header::InvalidHeaderValue),
    /// Unexpected response type.
    #[error("unexpected response type: {0}")]
    UnexpectedResponseType(anyhow::Error),
    /// Unsupported endpoint.
    #[error("unsuppored endpoint: {0}")]
    UnsupportedEndpoint(anyhow::Error),
    /// Need key.
    #[error("need apikey to sign the params")]
    NeedApikey,
    /// Sign error.
    #[error("sign error: {0}")]
    SignError(#[from] SignError),
    /// Utf-8 error.
    #[error("utf-8 error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    /// Text response.
    #[error("text response: {0}")]
    Text(String),
    /// Place Zero size.
    #[error("trying to place an order with zero size")]
    PlaceZeroSize,
    /// Failed to build exc symbol.
    #[error("failed to build exc symbol")]
    FailedToBuildExcSymbol,
    /// Missing date for futures.
    #[error("missing date for futures")]
    MissingDateForFutures,
    /// Invalid date for options.
    #[error("invalid date for options")]
    InvalidDateForOptions,
    /// Missing base asset for options.
    #[error("missing base asset for options")]
    MissingBaseAssetForOptions,
    /// Unknown contract type.
    #[error("unknown contract type: {0:?}")]
    UnknownContractType(String),
}

impl RestError {
    /// Is temp.
    pub fn is_temporary(&self) -> bool {
        if let Self::Exchange(err) = self {
            err.is_temporary()
        } else {
            false
        }
    }
}

/// Exchange Errors.
#[derive(Debug, Error)]
pub enum ExchangeError {
    /// Error from layers.
    #[error("layer: {0}")]
    Layer(#[from] Box<dyn std::error::Error + Send + Sync>),
    #[cfg(feature = "http")]
    /// Http errors.
    #[error("http: {0}")]
    Http(hyper::Error),
    /// All other errors.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
    /// All other api errors.
    #[error("api: {0}")]
    Api(anyhow::Error),
    /// Unavailable.
    #[error("unavailable: {0}")]
    Unavailable(anyhow::Error),
    /// Instrument errors.
    #[error("instrument: {0}")]
    Instrument(InstrumentError),
    /// Rate limited.
    #[error("rate limited: {0}")]
    RateLimited(anyhow::Error),
    /// API Key error.
    #[error("key error: {0}")]
    KeyError(anyhow::Error),
    /// Order not found.
    #[error("order not found")]
    OrderNotFound,
    /// Forbidden.
    #[error("forbidden: {0}")]
    Forbidden(anyhow::Error),
    /// Unexpected response type.
    #[error("unexpected response type: {0}")]
    UnexpectedResponseType(String),
}

impl ExchangeError {
    /// Is temporary.
    pub fn is_temporary(&self) -> bool {
        #[cfg(feature = "http")]
        {
            matches!(
                self,
                Self::RateLimited(_) | Self::Unavailable(_) | Self::Http(_)
            )
        }
        #[cfg(not(feature = "http"))]
        {
            matches!(self, Self::RateLimited(_) | Self::Unavailable(_))
        }
    }

    /// Flatten.
    pub fn flatten(self) -> Self {
        match self {
            Self::Layer(err) => match err.downcast::<Self>() {
                Ok(err) => (*err).flatten(),
                Err(err) => Self::Other(anyhow::anyhow!("{err}")),
            },
            err => err,
        }
    }

    /// Flatten layered error.
    pub fn layer(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        match err.downcast::<Self>() {
            Ok(err) => (*err).flatten(),
            Err(err) => Self::Other(anyhow::anyhow!("{err}")),
        }
    }

    /// Unexpected response type.
    pub fn unexpected_response_type(msg: impl ToString) -> Self {
        Self::UnexpectedResponseType(msg.to_string())
    }
}

impl From<Infallible> for ExchangeError {
    fn from(_: Infallible) -> Self {
        panic!("infallible")
    }
}
