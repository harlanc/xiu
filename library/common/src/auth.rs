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

pub enum SecretCarrier {
    Query(String),
    Bearer(String),
}

pub fn get_secret(carrier: &SecretCarrier) -> Result<String, AuthError> {
    match carrier {
        SecretCarrier::Query(query) => {
            let mut query_pairs = IndexMap::new();
            let pars_array: Vec<&str> = query.split('&').collect();
            for ele in pars_array {
                let (k, v) = scanf!(ele, '=', String, String);
                if k.is_none() || v.is_none() {
                    continue;
                }
                query_pairs.insert(k.unwrap(), v.unwrap());
            }

            query_pairs.get("token").map_or(
                Err(AuthError {
                    value: AuthErrorValue::NoTokenFound,
                }),
                |t| Ok(t.to_string()),
            )
        }
        SecretCarrier::Bearer(header) => {
            let invalid_format = Err(AuthError {
                value: AuthErrorValue::InvalidTokenFormat,
            });
            let (prefix, token) = scanf!(header, " ", String, String);

            //if prefix.is_none() || token.is_none() {
            //    invalid_format
            //} else if prefix.unwrap() != "Bearer" {
            //    invalid_format
            //} else {
            //    Ok(token.unwrap())
            //}
            //fix cargo clippy --fix --allow-dirty --allow-no-vcs warnings
            match token {
                Some(token_val) => match prefix {
                    Some(prefix_val) => {
                        if prefix_val != "Bearer" {
                            invalid_format
                        } else {
                            Ok(token_val)
                        }
                    }
                    None => invalid_format,
                },
                None => invalid_format,
            }
        }
    }
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
    push_password: Option<String>,
    pub auth_type: AuthType,
}

impl Auth {
    pub fn new(
        key: String,
        password: String,
        push_password: Option<String>,
        algorithm: AuthAlgorithm,
        auth_type: AuthType,
    ) -> Self {
        Self {
            algorithm,
            key,
            password,
            push_password,
            auth_type,
        }
    }

    pub fn authenticate(
        &self,
        stream_name: &String,
        secret: &Option<SecretCarrier>,
        is_pull: bool,
    ) -> Result<(), AuthError> {
        if self.auth_type == AuthType::Both
            || is_pull && (self.auth_type == AuthType::Pull)
            || !is_pull && (self.auth_type == AuthType::Push)
        {
            let mut auth_err_reason: String = String::from("there is no token str found.");
            let mut err: AuthErrorValue = AuthErrorValue::NoTokenFound;

            /*Here we should do auth and it must be successful. */
            if let Some(secret_value) = secret {
                let token = get_secret(secret_value)?;
                if self.check(stream_name, token.as_str(), is_pull) {
                    return Ok(());
                }
                auth_err_reason = format!("token is not correct: {}", token);
                err = AuthErrorValue::TokenIsNotCorrect;
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

    fn check(&self, stream_name: &String, auth_str: &str, is_pull: bool) -> bool {
        let password = if is_pull {
            &self.password
        } else {
            self.push_password.as_ref().unwrap_or(&self.password)
        };

        match self.algorithm {
            AuthAlgorithm::Simple => password == auth_str,
            AuthAlgorithm::Md5 => {
                let raw_data = format!("{}{}", self.key, stream_name);
                let digest_str = format!("{:x}", md5::compute(raw_data));
                auth_str == digest_str
            }
        }
    }
}
