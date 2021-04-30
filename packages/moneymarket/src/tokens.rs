use cosmwasm_bignumber::Uint256;
use cosmwasm_std::{Api, CanonicalAddr, Extern, HumanAddr, Querier, StdError, StdResult, Storage};

pub type Token = (CanonicalAddr, Uint256);
pub type TokenHuman = (HumanAddr, Uint256);

pub type Tokens = Vec<Token>;
pub type TokensHuman = Vec<TokenHuman>;

pub trait TokensMath {
    fn sub(&mut self, collaterals: Tokens) -> StdResult<()>;
    fn add(&mut self, collaterals: Tokens);
    fn assert_duplicate_token(&self);
}

pub trait TokensToHuman {
    fn to_human<S: Storage, A: Api, Q: Querier>(
        &self,
        deps: &Extern<S, A, Q>,
    ) -> StdResult<TokensHuman>;
}

pub trait TokensToRaw {
    fn to_raw<S: Storage, A: Api, Q: Querier>(&self, deps: &Extern<S, A, Q>) -> StdResult<Tokens>;
}

impl TokensMath for Tokens {
    fn sub(&mut self, tokens: Tokens) -> StdResult<()> {
        self.sort_by(|a, b| a.0.as_slice().cmp(&b.0.as_slice()));
        self.assert_duplicate_token();

        let mut tokens = tokens;
        tokens.sort_by(|a, b| a.0.as_slice().cmp(&b.0.as_slice()));
        tokens.assert_duplicate_token();

        let mut i = 0;
        let mut j = 0;
        while i < self.len() && j < tokens.len() {
            if self[i].0 == tokens[j].0 {
                if self[i].1 < tokens[j].1 {
                    return Err(StdError::generic_err("Subtraction underflow"));
                }

                self[i].1 = self[i].1 - tokens[j].1;

                i += 1;
                j += 1;
            } else if self[i].0.as_slice().cmp(&tokens[j].0.as_slice()) == std::cmp::Ordering::Less
            {
                i += 1;
            } else {
                return Err(StdError::generic_err("Subtraction underflow"));
            }
        }

        if j != tokens.len() {
            return Err(StdError::generic_err("Subtraction underflow"));
        }

        // remove zero tokens
        self.retain(|v| v.1 > Uint256::zero());

        Ok(())
    }

    fn add(&mut self, tokens: Tokens) {
        self.sort_by(|a, b| a.0.as_slice().cmp(&b.0.as_slice()));
        self.assert_duplicate_token();

        let mut tokens = tokens;
        tokens.sort_by(|a, b| a.0.as_slice().cmp(&b.0.as_slice()));
        tokens.assert_duplicate_token();

        let mut tmp_tokens: Tokens = vec![];

        let mut i = 0;
        let mut j = 0;
        while i < self.len() && j < tokens.len() {
            if self[i].0 == tokens[j].0 {
                tmp_tokens.push((self[i].0.clone(), self[i].1 + tokens[j].1));

                i += 1;
                j += 1;
            } else if self[i].0.as_slice().cmp(&tokens[j].0.as_slice())
                == std::cmp::Ordering::Greater
            {
                tmp_tokens.push((tokens[j].0.clone(), tokens[j].1));

                j += 1;
            } else {
                tmp_tokens.push((self[i].0.clone(), self[i].1));

                i += 1;
            }
        }

        while j < tokens.len() {
            tmp_tokens.push((tokens[j].0.clone(), tokens[j].1));
            j += 1;
        }

        while i < self.len() {
            tmp_tokens.push((self[i].0.clone(), self[i].1));
            i += 1;
        }

        // remove zero tokens
        tmp_tokens.retain(|v| v.1 > Uint256::zero());

        self.clear();
        self.extend(tmp_tokens);
    }

    fn assert_duplicate_token(&self) {
        if self.len() > 1 {
            let mut before_token = self[0].0.as_slice();

            let mut i = 1;
            while i < self.len() {
                let next_token = self[i].0.as_slice();
                if before_token == next_token {
                    panic!("duplicate token address");
                }

                before_token = next_token;
                i += 1;
            }
        }
    }
}

impl TokensToHuman for Tokens {
    fn to_human<S: Storage, A: Api, Q: Querier>(
        &self,
        deps: &Extern<S, A, Q>,
    ) -> StdResult<TokensHuman> {
        let collaterals: TokensHuman = self
            .iter()
            .map(|c| Ok((deps.api.human_address(&c.0)?, c.1)))
            .collect::<StdResult<TokensHuman>>()?;
        Ok(collaterals)
    }
}

impl TokensToRaw for TokensHuman {
    fn to_raw<S: Storage, A: Api, Q: Querier>(&self, deps: &Extern<S, A, Q>) -> StdResult<Tokens> {
        let collaterals: Tokens = self
            .iter()
            .map(|c| Ok((deps.api.canonical_address(&c.0)?, c.1)))
            .collect::<StdResult<Tokens>>()?;
        Ok(collaterals)
    }
}
