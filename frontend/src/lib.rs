use yew::prelude::*;
use wasm_bindgen_futures::spawn_local;
use serde::Deserialize;
use log;
use ws_stream_wasm::{WsMeta, WsMessage};
use futures::stream::StreamExt;
use wasm_bindgen::prelude::*;


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

/// The main application component.
#[function_component(App)]
fn app() -> Html {
    // State handle for the list of blocks.
    let blocks = use_state(|| vec![]);

    // Fetch the initial blocks via HTTP.
    {
        let blocks = blocks.clone();
        use_effect_with((), move |_| {
            let blocks_clone = blocks.clone();
            spawn_local(async move {
                match gloo_net::http::Request::get("http://127.0.0.1:8080/blocks").send().await {
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

    // Establish WebSocket connection for real-time updates.
    {
        let blocks = blocks.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                let (_ws, mut wsio) = WsMeta::connect("ws://127.0.0.1:8080/ws/", None)
                    .await
                    .expect("Failed to connect to WebSocket");

                log::info!("WebSocket connection established.");

                while let Some(msg) = wsio.next().await {
                    if let WsMessage::Text(txt) = msg {
                        log::info!("Received WebSocket message: {}", txt);
                        match serde_json::from_str::<Block>(&txt) {
                            Ok(new_block) => {
                                let mut current_blocks = (*blocks).clone();
                                current_blocks.push(new_block);
                                blocks.set(current_blocks);
                            }
                            Err(e) => log::error!("Failed to deserialize block from WebSocket: {:?}", e),
                        }
                    }
                }
                log::info!("WebSocket connection closed.");
            });

            || ()
        });
    }

    if blocks.is_empty() {
        html! {
            <div>
                <h1>{ "SierpChain" }</h1>
                <p>{ "Loading blocks..." }</p>
            </div>
        }
    } else {
        html! {
            <div>
                <h1>{ "SierpChain" }</h1>
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
#[wasm_bindgen(start)]
pub fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();
}
