#![allow(dead_code)]
use std::sync::Arc;

use anonymous_credit_tokens::{
    CreditToken, IssuanceRequest, IssuanceResponse, Params, PreIssuance, PreRefund, PublicKey,
    Refund, SpendProof,
};
use curve25519_dalek::Scalar;
use rand_core::OsRng;
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    handler::server::{
        router::{prompt::PromptRouter, tool::ToolRouter},
        wrapper::Parameters,
    },
    model::*,
    prompt, prompt_handler, prompt_router, schemars,
    service::RequestContext,
    tool, tool_handler, tool_router,
};
use serde_json::json;
use tokio::sync::Mutex;

fn get_params() -> Params {
    Params::new("example-org", "payment-api", "production", "2024-01-15")
}

pub struct PreReqCbor {
    pub preissuance: Vec<u8>,
    pub issuance_request: Vec<u8>,
}

pub fn request_credits() -> PreReqCbor {
    let preissuance = PreIssuance::random(OsRng);
    let issuance_request = preissuance.request(&get_params(), OsRng);
    PreReqCbor {
        preissuance: preissuance.to_cbor().unwrap(),
        issuance_request: issuance_request.to_cbor().unwrap(),
    }
}

pub fn finalize_credits(
    public_key_cbor: &Vec<u8>,
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

    return credit_token.to_cbor().unwrap();
}

pub struct PreSpendCbor {
    pub spend_proof: Vec<u8>,
    pub prerefund: Vec<u8>,
}

pub fn spend_tokens(n: u8, credit_token_cbor: Vec<u8>) -> PreSpendCbor {
    let charge = Scalar::from(n);
    let credit_token = CreditToken::from_cbor(&credit_token_cbor).unwrap();

    let (spend_proof, prerefund) = credit_token.prove_spend(&get_params(), charge, OsRng);
    PreSpendCbor {
        spend_proof: spend_proof.to_cbor().unwrap(),
        prerefund: prerefund.to_cbor().unwrap(),
    }
}

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
        .to_credit_token(&get_params(), &spend_proof, &refund, &public_key)
        .unwrap();
    credit_token.to_cbor().unwrap()
}

#[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct ExamplePromptArgs {
    // pub url: String,
}

#[derive(Clone)]
pub struct ACTFetcher {
    credits: Arc<Mutex<i32>>,
    login: Arc<Mutex<Vec<u8>>>,
    public_key: Arc<Mutex<Vec<u8>>>,
    tool_router: ToolRouter<ACTFetcher>,
    prompt_router: PromptRouter<ACTFetcher>,
}

#[tool_router]
impl ACTFetcher {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            credits: Arc::new(Mutex::new(0)),
            login: Arc::new(Mutex::new(Vec::new())),
            public_key: Arc::new(Mutex::new(Vec::new())),
            tool_router: Self::tool_router(),
            prompt_router: Self::prompt_router(),
        }
    }

    fn _create_resource_text(&self, uri: &str, name: &str) -> Resource {
        RawResource::new(uri, name.to_string()).no_annotation()
    }

    #[tool(description = "Retrieve an ACT credential from the local issuer")]
    async fn act_issue(&self) -> Result<CallToolResult, McpError> {
        let client = reqwest::Client::new();

        let public_key_cbor = client.get("http://localhost:8787/public")
            .send()
            .await.unwrap()
            .bytes()
            .await.unwrap()
            .to_vec();

        let pre = request_credits();
        let credits: u8 = 3;

        let body: Vec<u8> = vec![credits]
            .into_iter()
            .chain(pre.issuance_request.clone())
            .collect();

        let issuance_response_cbor = client.post("http://localhost:8787/request")
            .body(body)
            .send()
            .await.unwrap()
            .bytes()
            .await.unwrap()
            .to_vec();

        let credit_login = finalize_credits(&public_key_cbor, pre, issuance_response_cbor);

        let mut credits = self.credits.lock().await;
        *credits = 3;
        let mut login = self.login.lock().await;
        *login = credit_login;
        let mut public_key = self.public_key.lock().await;
        *public_key = public_key_cbor;

        Ok(CallToolResult::success(vec![Content::text(
            "ACT issuance successful. You now have 3 credits.",
        )]))
    }

    #[tool(description="Fetch mini-ai-agent.cloudflareresearch.com/llm with ACT credentials")]
    async fn act_redeem(&self) -> Result<CallToolResult, McpError> {
        let mut login = self.login.lock().await;
        let client = reqwest::Client::new();


        let pre = spend_tokens(1, login.clone());
        let body = pre.spend_proof.clone();

        let refund = client.post("http://localhost:8787/spend")
            .body(body)
            .send()
            .await.unwrap()
            .bytes()
            .await.unwrap()
            .to_vec();

        let public_key = self.public_key.lock().await;
        let refund = update_refund(pre, refund, public_key.clone());

        let mut credits = self.credits.lock().await;
        *credits -= 1;
        *login = refund;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Current credit amount {}",
            *credits
        ))]))
    }
}

#[prompt_router]
impl ACTFetcher {
    /// This is an example prompt that can access mini-ai-agent.cloudflareresearch.com/llm.
    /// It uses ACT when making requests, and fetches the right credential if you don't have it.
    #[prompt(name = "act_example_prompt")]
    async fn example_prompt(
        &self,
        Parameters(_args): Parameters<ExamplePromptArgs>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<Vec<PromptMessage>, McpError> {
        // here we write a prompt that will login if the current credit amount is zero
        // once that's done, it will access mini-ai-agent.cloudflareresearch.com/llm
        let prompt = format!(
            "You are an agent that can access mini-ai-agent.cloudflareresearch.com/llm. You have ACT credentials to pay for access. If you don't have any credits, first login to get more credits. Then access mini-ai-agent.cloudflareresearch.com/llm.",
            // args.url
        );
        Ok(vec![
            PromptMessage::new_text(
                PromptMessageRole::Assistant,
                "I'll first check if I have a credential, then access mini-ai-agent.cloudflareresearch.com/llm.",
            ),
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(prompt),
            }
        ])
    }
}

#[tool_handler]
#[prompt_handler]
impl ServerHandler for ACTFetcher {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_prompts()
                .enable_resources()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some("This server provides counter tools and prompts. Tools: increment, decrement, get_value, say_hello, echo, sum. Prompts: example_prompt (takes a message), counter_analysis (analyzes counter state with a goal).".to_string()),
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(ListResourcesResult {
            resources: vec![
                self._create_resource_text("str:////Users/to/some/path/", "cwd"),
                self._create_resource_text("memo://insights", "memo-name"),
            ],
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        ReadResourceRequestParam { uri }: ReadResourceRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        match uri.as_str() {
            "str:////Users/to/some/path/" => {
                let cwd = "/Users/to/some/path/";
                Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(cwd, uri)],
                })
            }
            "memo://insights" => {
                let memo = "Business Intelligence Memo\n\nAnalysis has revealed 5 key insights ...";
                Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(memo, uri)],
                })
            }
            _ => Err(McpError::resource_not_found(
                "resource_not_found",
                Some(json!({
                    "uri": uri
                })),
            )),
        }
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        Ok(ListResourceTemplatesResult {
            next_cursor: None,
            resource_templates: Vec::new(),
        })
    }

    async fn initialize(
        &self,
        _request: InitializeRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, McpError> {
        if let Some(http_request_part) = context.extensions.get::<axum::http::request::Parts>() {
            let initialize_headers = &http_request_part.headers;
            let initialize_uri = &http_request_part.uri;
            tracing::info!(?initialize_headers, %initialize_uri, "initialize from http server");
        }
        Ok(self.get_info())
    }
}