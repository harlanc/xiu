use indexmap::IndexMap;
use md5;
use serde_derive::Deserialize;

use crate::errors::{AuthError, AuthErrorValue};
use crate::scanf;

#[derive(Debug, Deserialize, Clone, Default)]
pub enum AuthAlgorithm {
    #[default]
    #[serde(rename = "simple")]
    Simple,
    #[serde(rename = "md5")]
    Md5,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AuthType {
    Pull,
    Push,
    Both,
    None,
}
#[derive(Debug, Clone)]
pub struct Auth {
    algorithm: AuthAlgorithm,
    key: String,
    password: String,
    pub auth_type: AuthType,
}

impl Auth {
    pub fn new(
        key: String,
        password: String,
        algorithm: AuthAlgorithm,
        auth_type: AuthType,
    ) -> Self {
        Self {
            algorithm,
            key,
            password,
            auth_type,
        }
    }

    pub fn authenticate(
        &self,
        stream_name: &String,
        query: &Option<String>,
        is_pull: bool,
    ) -> Result<(), AuthError> {
        if self.auth_type == AuthType::Both
            || is_pull && (self.auth_type == AuthType::Pull)
            || !is_pull && (self.auth_type == AuthType::Push)
        {
            let mut auth_err_reason: String = String::from("there is no token str found.");
            let mut err: AuthErrorValue = AuthErrorValue::NoTokenFound;

            /*Here we should do auth and it must be successful. */
            if let Some(query_val) = query {
                let mut query_pairs = IndexMap::new();
                let pars_array: Vec<&str> = query_val.split('&').collect();
                for ele in pars_array {
                    let (k, v) = scanf!(ele, '=', String, String);
                    if k.is_none() || v.is_none() {
                        continue;
                    }
                    query_pairs.insert(k.unwrap(), v.unwrap());
                }

                if let Some(token) = query_pairs.get("token") {
                    if self.check(stream_name, token) {
                        return Ok(());
                    }
                    auth_err_reason = format!("token is not correct: {}", token);
                    err = AuthErrorValue::TokenIsNotCorrect;
                }
            }

            log::error!(
                "Auth error stream_name: {} auth type: {:?} pull: {} reason: {}",
                stream_name,
                self.auth_type,
                is_pull,
                auth_err_reason,
            );
            return Err(AuthError { value: err });
        }
        Ok(())
    }

    fn check(&self, stream_name: &String, auth_str: &str) -> bool {
        match self.algorithm {
            AuthAlgorithm::Simple => {
                self.password == auth_str
            }
            AuthAlgorithm::Md5 => {
                let raw_data = format!("{}{}", self.key, stream_name);
                let digest_str = format!("{:x}", md5::compute(raw_data));
                auth_str == digest_str
            }
        }
    }
}
