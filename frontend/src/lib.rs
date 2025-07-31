use yew::prelude::*;
use gloo_net::http::Request;
use wasm_bindgen_futures::spawn_local;
use serde::{Deserialize, Serialize};
use log;
use web_sys;
use yew::events::SubmitEvent;

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
#[derive(Clone, PartialEq, Deserialize, Debug)]
pub struct Block {
    pub index: u64,
    pub timestamp: i64,
    pub fractal: FractalTriangle,
    pub data: String,
    pub previous_hash: String,
    pub hash: String,
    pub nonce: u64,
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
        <svg viewBox="-0.1 -0.1 1.2 1.2" width="100" height="100">
            <g>
                { for points_list.iter().map(|points| html!{
                    <polygon points={points.clone()} fill="black" />
                })}
            </g>
        </svg>
    }
}

#[function_component(WalletComponent)]
fn wallet_component() -> Html {
    let wallet_info = use_state(|| None);
    let to_address = use_state(String::new);
    let amount = use_state(|| 0);

    // Fetch wallet info
    {
        let wallet_info = wallet_info.clone();
        use_effect_with((), move |_| {
            let wallet_info = wallet_info.clone();
            spawn_local(async move {
                match Request::get("http://127.0.0.1:8080/wallet/info").send().await {
                    Ok(response) => {
                        if response.ok() {
                            match response.json::<WalletInfo>().await {
                                Ok(info) => wallet_info.set(Some(info)),
                                Err(e) => log::error!("Failed to deserialize wallet info: {:?}", e),
                            }
                        } else {
                            log::error!("Failed to fetch wallet info: status {}", response.status());
                        }
                    }
                    Err(e) => log::error!("Failed to send request: {:?}", e),
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
                match Request::post("http://127.0.0.1:8080/transact")
                    .json(&req)
                    .unwrap()
                    .send()
                    .await
                {
                    Ok(response) => {
                        if response.ok() {
                            log::info!("Transaction successful");
                            // Optionally refresh wallet info or block list
                        } else {
                            log::error!("Transaction failed: status {}", response.status());
                        }
                    }
                    Err(e) => log::error!("Failed to send transaction request: {:?}", e),
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
            <div class="wallet-card" style="border: 1px solid black; padding: 10px; margin: 10px;">
                <h2>{ "My Wallet" }</h2>
                <p>{ format!("Address: {}...", info.address.chars().take(30).collect::<String>()) }</p>
                <p>{ format!("Balance: {}", info.balance) }</p>
                <form onsubmit={on_submit}>
                    <h3>{ "Send Funds" }</h3>
                    <div>
                        <label for="to_address">{ "To Address:" }</label>
                        <input type="text" id="to_address" value={(*to_address).clone()} onchange={on_to_address_change} style="width: 90%"/>
                    </div>
                    <div>
                        <label for="amount">{ "Amount:" }</label>
                        <input type="number" id="amount" value={amount.to_string()} onchange={on_amount_change}/>
                    </div>
                    <button type="submit">{ "Send" }</button>
                </form>
            </div>
        }
    } else {
        html! { <p>{ "Loading wallet..." }</p> }
    }
}

/// The main application component.
#[function_component(App)]
fn app() -> Html {
    // State handle for the list of blocks.
    let blocks = use_state(|| vec![]);

    {
        let blocks = blocks.clone();
        // Fetch the blocks from the backend when the component is first rendered.
        use_effect_with((), move |_| {
            let blocks_clone = blocks.clone();
            spawn_local(async move {
                match Request::get("http://127.0.0.1:8080/blocks").send().await {
                    Ok(response) => {
                        if response.ok() {
                            match response.json::<Vec<Block>>().await {
                                Ok(fetched_blocks) => {
                                    blocks_clone.set(fetched_blocks);
                                }
                                Err(e) => log::error!("Failed to deserialize blocks: {:?}", e),
                            }
                        } else {
                            log::error!("Failed to fetch blocks: status {}", response.status());
                        }
                    }
                    Err(e) => log::error!("Failed to send request: {:?}", e),
                }
            });
            || ()
        });
    }

    if blocks.is_empty() {
        html! {
            <div>
                <h1>{ "SierpChain" }</h1>
                <WalletComponent />
                <p>{ "Loading blocks..." }</p>
            </div>
        }
    } else {
        html! {
            <div>
                <h1>{ "SierpChain" }</h1>
                <WalletComponent />
                <div class="blocks-container">
                    { for blocks.iter().map(|block| html! {
                        <div class="block-card" style="border: 1px solid black; padding: 10px; margin: 10px;">
                            <h2>{ format!("Block #{}", block.index) }</h2>
                            <p>{ format!("Hash: {}...", block.hash.chars().take(20).collect::<String>()) }</p>
                            <p>{ format!("Previous Hash: {}...", block.previous_hash.chars().take(20).collect::<String>()) }</p>
                            <p>{ format!("Nonce: {}", block.nonce) }</p>
                            <p>{ format!("Data: {}", block.data) }</p>
                            <p>{ format!("Fractal Depth: {}", block.fractal.depth) }</p>
                            <FractalTriangleComponent triangle={block.fractal.clone()} />
                        </div>
                    })}
                </div>
            </div>
        }
    }
}

/// The main entry point for the frontend application.
fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();
}
