use cosmwasm_std::{
    Api, CanonicalAddr, Extern, HumanAddr, Querier, StdError, StdResult, Storage, Uint128,
};

pub type Token = (CanonicalAddr, Uint128);
pub type TokenHuman = (HumanAddr, Uint128);

pub type Tokens = Vec<Token>;
pub type TokensHuman = Vec<TokenHuman>;

pub trait TokensMath {
    fn sub(&mut self, collaterals: Tokens) -> StdResult<()>;
    fn add(&mut self, collaterals: Tokens);
}

pub trait TokensToHuman {
    fn to_human<S: Storage, A: Api, Q: Querier>(
        &self,
        deps: &Extern<S, A, Q>,
    ) -> StdResult<TokensHuman>;
}

pub trait TokensToRaw {
    fn to_raw<S: Storage, A: Api, Q: Querier>(
        &self,
        deps: &Extern<S, A, Q>,
    ) -> StdResult<Tokens>;
}

impl TokensMath for Tokens {
    fn sub(&mut self, tokens: Tokens) -> StdResult<()> {
        self.sort_by(|a, b| a.0.as_slice().cmp(&b.0.as_slice()));

        let mut tokens = tokens;
        tokens.sort_by(|a, b| a.0.as_slice().cmp(&b.0.as_slice()));

        let mut i = 0;
        let mut j = 0;
        while i < self.len() && j < tokens.len() {
            if self[i].0 == tokens[j].0 {
                self[i].1 = (self[i].1 - tokens[j].1)?;

                i += 1;
                j += 1;
            } else if self[i].0.as_slice().cmp(&tokens[j].0.as_slice())
                == std::cmp::Ordering::Greater
            {
                j += 1;
            } else {
                i += 1;
            }
        }

        if j != tokens.len() {
            return Err(StdError::generic_err("Subtraction underflow"));
        }

        Ok(())
    }

    fn add(&mut self, tokens: Tokens) {
        self.sort_by(|a, b| a.0.as_slice().cmp(&b.0.as_slice()));

        let mut tokens = tokens;
        tokens.sort_by(|a, b| a.0.as_slice().cmp(&b.0.as_slice()));

        let mut i = 0;
        let mut j = 0;
        while i < self.len() && j < tokens.len() {
            if self[i].0 == tokens[j].0 {
                self[i].1 += tokens[j].1;

                i += 1;
                j += 1;
            } else if self[i].0.as_slice().cmp(&tokens[j].0.as_slice())
                == std::cmp::Ordering::Greater
            {
                j += 1;
            } else {
                i += 1;
            }
        }

        while j < tokens.len() {
            self.push(tokens[j].clone());
            j += 1;
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
    fn to_raw<S: Storage, A: Api, Q: Querier>(
        &self,
        deps: &Extern<S, A, Q>,
    ) -> StdResult<Tokens> {
        let collaterals: Tokens = self
            .iter()
            .map(|c| Ok((deps.api.canonical_address(&c.0)?, c.1)))
            .collect::<StdResult<Tokens>>()?;
        Ok(collaterals)
    }
}
