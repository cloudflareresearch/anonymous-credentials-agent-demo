// Copyright 2025 Cloudflare, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use anonymous_credit_tokens::{
    CreditToken, IssuanceRequest, IssuanceResponse, Params, PreIssuance, PreRefund, PublicKey,
    Refund, SpendProof,
};
use curve25519_dalek::Scalar;
use rand_core::OsRng;
use wasm_bindgen::prelude::*;

fn get_params() -> Params {
    Params::new("example-org", "payment-api", "production", "2024-01-15")
}

#[wasm_bindgen(getter_with_clone)]
pub struct PreReqCbor {
    #[wasm_bindgen(readonly)]
    pub preissuance: Vec<u8>,
    #[wasm_bindgen(readonly)]
    pub issuance_request: Vec<u8>,
}

#[wasm_bindgen]
pub fn request_credits() -> PreReqCbor {
    let preissuance = PreIssuance::random(OsRng);
    let issuance_request = preissuance.request(&get_params(), OsRng);
    PreReqCbor {
        preissuance: preissuance.to_cbor().unwrap(),
        issuance_request: issuance_request.to_cbor().unwrap(),
    }
}

#[wasm_bindgen]
pub fn finalize_credits(
    public_key_cbor: Vec<u8>,
    pre_req: PreReqCbor,
    issuance_response_cbor: Vec<u8>,
) -> Vec<u8> {
    let preissuance = PreIssuance::from_cbor(&pre_req.preissuance).unwrap();
    let public_key = PublicKey::from_cbor(&public_key_cbor).unwrap();
    let issuance_request = IssuanceRequest::from_cbor(&pre_req.issuance_request).unwrap();
    let issuance_response = IssuanceResponse::from_cbor(&issuance_response_cbor).unwrap();

    let credit_token = preissuance
        .to_credit_token(
            &get_params(),
            &public_key,
            &issuance_request,
            &issuance_response,
        )
        .unwrap();

    credit_token.to_cbor().unwrap()
}

#[wasm_bindgen(getter_with_clone)]
pub struct PreSpendCbor {
    #[wasm_bindgen(readonly)]
    pub spend_proof: Vec<u8>,
    #[wasm_bindgen(readonly)]
    pub prerefund: Vec<u8>,
}

#[wasm_bindgen]
pub fn spend_tokens(n: u8, credit_token_cbor: Vec<u8>) -> PreSpendCbor {
    let charge = Scalar::from(n);
    let credit_token = CreditToken::from_cbor(&credit_token_cbor).unwrap();

    let (spend_proof, prerefund) = credit_token.prove_spend(&get_params(), charge, OsRng);
    PreSpendCbor {
        spend_proof: spend_proof.to_cbor().unwrap(),
        prerefund: prerefund.to_cbor().unwrap(),
    }
}

#[wasm_bindgen]
pub fn update_refund(
    pre_spend: PreSpendCbor,
    refund_cbor: Vec<u8>,
    public_key_cbor: Vec<u8>,
) -> Vec<u8> {
    let prerefund = PreRefund::from_cbor(&pre_spend.prerefund).unwrap();
    let spend_proof = SpendProof::from_cbor(&pre_spend.spend_proof).unwrap();
    let refund = Refund::from_cbor(&refund_cbor).unwrap();
    let public_key = PublicKey::from_cbor(&public_key_cbor).unwrap();

    let credit_token = prerefund
        .to_credit_token(&spend_proof, &refund, &public_key)
        .unwrap();
    credit_token.to_cbor().unwrap()
}
