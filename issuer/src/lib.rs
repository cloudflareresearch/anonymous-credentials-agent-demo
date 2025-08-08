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

use anonymous_credit_tokens::{IssuanceRequest, Params, PrivateKey, SpendProof};
use curve25519_dalek::Scalar;
use hex;
use rand_core::OsRng;
use worker::*;

struct NullifierStore {}

impl NullifierStore {
    pub fn is_used(&self, _nullifier: &Scalar) -> bool {
        return false;
    }
    pub fn mark_used(&mut self, _nullifier: Scalar) {}
}

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();
    Router::new()
        .get("/public", serve_public_key)
        .post_async("/request", serve_request)
        .post_async("/spend", serve_spend)
        .run(req, env)
        .await
}

fn wrap(mut r: Response) -> Result<Response> {
    let cors_headers = [
        ("Access-Control-Allow-Origin", "*"),
        ("Access-Control-Allow-Methods", "GET,HEAD,POST,OPTIONS"),
        ("Access-Control-Max-Age", "86400"),
    ];
    let headers = r.headers_mut();
    for (h, v) in cors_headers.iter() {
        headers.set(h, v)?;
    }
    Ok(r)
}

fn get_params() -> Params {
    Params::new("example-org", "payment-api", "production", "2024-01-15")
}

fn fetch_private_key(env: &Env) -> Result<PrivateKey> {
    const BINDING: &str = "ISSUER_PRIVATEKEY_CBOR_HEX";
    let secret = env.secret(BINDING)?;
    let sk_cbor_hex = secret
        .as_ref()
        .as_string()
        .ok_or(Error::from(format!("{BINDING} not found")))?;
    let sk_cbor = hex::decode(sk_cbor_hex).map_err(|e| e.to_string())?;
    let sk = PrivateKey::from_cbor(&sk_cbor).map_err(|e| Error::from(format!("{:?}", e)))?;
    Ok(sk)
}

pub fn serve_public_key(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let private_key = fetch_private_key(&ctx.env)?;
    let public_key = private_key.public();
    wrap(Response::from_bytes(public_key.to_cbor().unwrap())?)
}

pub async fn serve_request(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let bytes = req.bytes().await.unwrap();
    let credits = bytes[0];
    let cred_req_cbor = &bytes[1..];
    let issuance_request = IssuanceRequest::from_cbor(cred_req_cbor).unwrap();
    let private_key = fetch_private_key(&ctx.env)?;
    let issuance_response = private_key
        .issue(
            &get_params(),
            &issuance_request,
            Scalar::from(credits),
            OsRng,
        )
        .unwrap();
    wrap(Response::from_bytes(issuance_response.to_cbor().unwrap())?)
}

pub async fn serve_spend(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let bytes = req.bytes().await.unwrap();
    let spend_proof = SpendProof::from_cbor(&bytes).unwrap();

    // todo: implement double spending detection.
    let mut nullifier_store = NullifierStore {};
    let nullifier = spend_proof.nullifier();
    if nullifier_store.is_used(&nullifier) {
        return wrap(Response::error(
            "err: Double-spending attempt detected",
            400,
        )?);
    }
    nullifier_store.mark_used(nullifier);

    let private_key = fetch_private_key(&ctx.env)?;
    match private_key.refund(&get_params(), &spend_proof, OsRng) {
        Ok(refund) => wrap(Response::from_bytes(refund.to_cbor().unwrap())?),
        Err(e) => wrap(Response::error(
            format!("err: not enough credits available to spend: {:?}", e),
            400,
        )?),
    }
}
