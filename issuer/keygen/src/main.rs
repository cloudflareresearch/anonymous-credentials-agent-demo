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

use anonymous_credit_tokens::PrivateKey;
use rand_core::OsRng;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let private_key = PrivateKey::random(OsRng);
    let private_key_cbor = private_key
        .to_cbor()
        .map_err(|e| format!("error: {:?}", e))?;
    let private_key_cbor_hex = hex::encode(private_key_cbor);

    let args: Vec<String> = std::env::args().collect();
    let default_output_file = String::from("issuer_key.txt");
    let output_file = args.get(1).unwrap_or(&default_output_file);
    std::fs::write(output_file, private_key_cbor_hex)?;
    println!("Issuer's private key written at {output_file} file.");
    Ok(())
}
