use yew::prelude::*;
use gloo_net::http::Request;
use wasm_bindgen_futures::spawn_local;
use serde::{Deserialize, Serialize};
use log;
use web_sys;
use yew::events::SubmitEvent;
use gloo_net::websocket::{WebSocket, Message as WsMessage};
use futures::stream::StreamExt;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct TransactRequest {
    to: String,
    amount: u64,
}

#[derive(Clone, PartialEq, Deserialize, Debug)]
pub struct WalletInfo {
    pub address: String,
    pub balance: u64,
}

/// Represents a Sierpinski triangle fractal.
#[derive(Clone, PartialEq, Deserialize, Debug)]
pub struct FractalTriangle {
    pub depth: usize,
    pub vertices: Vec<(f64, f64)>,
}

/// Represents a block in the SierpChain.
#[derive(Clone, PartialEq, Deserialize, Debug, Default)]
pub struct Block {
    pub index: u64,
    pub timestamp: i64,
    pub fractal: FractalTriangle,
    pub transactions: Vec<Transaction>,
    pub previous_hash: String,
    pub hash: String,
    pub nonce: u64,
}

/// Represents a transaction.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Transaction {
    pub id: String,
    pub timestamp: i64,
    pub inputs: Vec<TxInput>,
    pub outputs: Vec<TxOutput>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TxInput {
    pub txid: String,
    pub vout: usize,
    pub script_sig: String,
    pub pub_key: String,
    pub sequence: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TxOutput {
    pub value: u64,
    pub script_pub_key: String,
}

impl Default for FractalTriangle {
    fn default() -> Self {
        Self { depth: 0, vertices: vec![] }
    }
}

/// Properties for the `FractalTriangleComponent`.
#[derive(Properties, PartialEq)]
pub struct FractalTriangleProps {
    pub triangle: FractalTriangle,
}

/// A Yew component for rendering a `FractalTriangle` as an SVG.
#[function_component(FractalTriangleComponent)]
fn fractal_triangle_component(props: &FractalTriangleProps) -> Html {
    let points_list = props.triangle.vertices.chunks(3).map(|chunk| {
        format!("{},{} {},{} {},{}", chunk[0].0, chunk[0].1, chunk[1].0, chunk[1].1, chunk[2].0, chunk[2].1)
    }).collect::<Vec<String>>();

    html! {
        <div class="fractal-container">
            <svg viewBox="-0.1 -0.1 1.2 1.2">
                <g>
                    { for points_list.iter().map(|points| html!{
                        <polygon points={points.clone()} />
                    })}
                </g>
            </svg>
        </div>
    }
}

#[function_component(WalletComponent)]
fn wallet_component() -> Html {
    let wallet_info = use_state(|| None);
    let to_address = use_state(String::new);
    let amount = use_state(|| 0);

    {
        let wallet_info = wallet_info.clone();
        use_effect_with((), move |_| {
            let wallet_info = wallet_info.clone();
            spawn_local(async move {
                if let Ok(response) = Request::get("http://127.0.0.1:8080/wallet/info").send().await {
                    if response.ok() {
                        if let Ok(info) = response.json::<WalletInfo>().await {
                            wallet_info.set(Some(info));
                        }
                    }
                }
            });
            || ()
        });
    }

    let on_submit = {
        let to_address = to_address.clone();
        let amount = amount.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let to = (*to_address).clone();
            let amnt = *amount;
            spawn_local(async move {
                let req = TransactRequest { to, amount: amnt };
                if let Ok(response) = Request::post("http://127.0.0.1:8080/transact").json(&req).unwrap().send().await {
                    if response.ok() {
                        log::info!("Transaction successful");
                    } else {
                        log::error!("Transaction failed");
                    }
                }
            });
        })
    };

    let on_to_address_change = {
        let to_address = to_address.clone();
        Callback::from(move |e: Event| {
            let value = e.target_unchecked_into::<web_sys::HtmlInputElement>().value();
            to_address.set(value);
        })
    };

    let on_amount_change = {
        let amount = amount.clone();
        Callback::from(move |e: Event| {
            let value = e.target_unchecked_into::<web_sys::HtmlInputElement>().value();
            amount.set(value.parse().unwrap_or(0));
        })
    };

    if let Some(info) = &*wallet_info {
        html! {
            <div class="wallet-card">
                <h2>{ "My Wallet" }</h2>
                <p><strong>{ "Address: " }</strong>{ &info.address }</p>
                <p><strong>{ "Balance: " }</strong>{ info.balance }</p>
                <form onsubmit={on_submit}>
                    <h3>{ "Send Funds" }</h3>
                    <div>
                        <label for="to_address">{ "To Address:" }</label>
                        <input type="text" id="to_address" value={(*to_address).clone()} onchange={on_to_address_change} />
                    </div>
                    <div>
                        <label for="amount">{ "Amount:" }</label>
                        <input type="number" id="amount" value={amount.to_string()} onchange={on_amount_change} />
                    </div>
                    <button type="submit">{ "Send" }</button>
                </form>
            </div>
        }
    } else {
        html! { <div class="wallet-card"><p>{ "Loading wallet..." }</p></div> }
    }
}

/// The main application component.
#[function_component(App)]
fn app() -> Html {
    let blocks = use_state(|| vec![]);
    let _ws_task = use_state(|| None);

    {
        let blocks = blocks.clone();
        use_effect_with((), move |_| {
            let blocks = blocks.clone();
            spawn_local(async move {
                if let Ok(response) = Request::get("http://127.0.0.1:8080/blocks").send().await {
                    if response.ok() {
                        if let Ok(fetched_blocks) = response.json::<Vec<Block>>().await {
                            blocks.set(fetched_blocks);
                        }
                    }
                }
            });
            || ()
        });
    }

    {
        let blocks = blocks.clone();
        let ws_task_handle = _ws_task.clone();
        use_effect_with((), move |_| {
            let ws_conn = WebSocket::open("ws://127.0.0.1:8080/ws").unwrap();
            let (mut _write, mut read) = ws_conn.split();

            let ws_task = spawn_local(async move {
                while let Some(Ok(WsMessage::Text(data))) = read.next().await {
                    if let Ok(new_block) = serde_json::from_str::<Block>(&data) {
                        let mut updated_blocks = (*blocks).clone();
                        updated_blocks.push(new_block);
                        blocks.set(updated_blocks);
                    }
                }
            });
            ws_task_handle.set(Some(ws_task));
            || ()
        });
    }

    html! {
        <div>
            <h1>{ "SierpChain 🔺⛓️" }</h1>
            <div class="app-container">
                <div class="sidebar">
                    <WalletComponent />
                </div>
                <div class="main-content">
                    if blocks.is_empty() {
                        <p>{ "Loading blocks..." }</p>
                    } else {
                        <div class="blocks-container">
                            { for blocks.iter().rev().map(|block| html! {
                                <div class="block-card">
                                    <FractalTriangleComponent triangle={block.fractal.clone()} />
                                    <div class="block-details">
                                        <h2>{ format!("Block #{}", block.index) }</h2>
                                        <p><strong>{ "Hash: " }</strong>{ &block.hash }</p>
                                        <p><strong>{ "Prev. Hash: " }</strong>{ &block.previous_hash }</p>
                                        <p><strong>{ "Nonce: " }</strong>{ block.nonce }</p>
                                        <p><strong>{ "Transactions: " }</strong>{ block.transactions.len() }</p>
                                        <p><strong>{ "Fractal Depth: " }</strong>{ block.fractal.depth }</p>
                                    </div>
                                </div>
                            })}
                        </div>
                    }
                </div>
            </div>
        </div>
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();
}
