// #[derive(Debug)]
// enum ApiError {
//     RequestFailed(reqwest::Error),
//     BadStatus(StatusCode),
//     ParseError(String),
// }

// impl std::fmt::Display for ApiError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             ApiError::RequestFailed(err) => write!(f, "Request failed: {}", err),
//             ApiError::BadStatus(status) => write!(f, "Bad status code: {}", status),
//             ApiError::ParseError(msg) => write!(f, "Parse error: {}", msg),
//         }
//     }
// }

// impl std::error::Error for ApiError {}
