use yew::prelude::*;
use gloo_net::http::Request;
use wasm_bindgen_futures::spawn_local;
use serde::{Deserialize, Serialize};
use log;
use web_sys;
use yew::events::SubmitEvent;
use gloo_net::websocket::{Message as WsMessage};
use gloo_net::websocket::futures::WebSocket;
use futures::stream::StreamExt;
use web_sys::wasm_bindgen::{JsCast, Clamped};
use serde_json;

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

#[derive(Clone, PartialEq, Deserialize, Debug)]
pub struct Sierpinski {
    pub depth: usize,
    pub seed: u64,
    pub vertices: Vec<(f64, f64)>,
}

#[derive(Clone, PartialEq, Deserialize, Debug)]
pub struct Mandelbrot {
    pub width: usize,
    pub height: usize,
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
    pub max_iterations: u32,
    pub seed: u64,
    pub data: Vec<u32>,
}

#[derive(Clone, PartialEq, Deserialize, Debug)]
pub struct Julia {
    pub width: usize,
    pub height: usize,
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
    pub c_real: f64,
    pub c_imag: f64,
    pub max_iterations: u32,
    pub seed: u64,
    pub data: Vec<u32>,
}

#[derive(Clone, PartialEq, Deserialize, Debug)]
#[serde(tag = "type", content = "data")]
pub enum FractalData {
    Sierpinski(Sierpinski),
    Mandelbrot(Mandelbrot),
    Julia(Julia),
}

/// Represents a block in the SierpChain.
#[derive(Clone, PartialEq, Deserialize, Debug)]
pub struct Block {
    pub index: u64,
    pub timestamp: i64,
    pub fractal: FractalData,
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

/// Properties for the `SierpinskiComponent`.
#[derive(Properties, PartialEq)]
pub struct SierpinskiProps {
    pub sierpinski: Sierpinski,
}

/// A Yew component for rendering a `Sierpinski` as an SVG.
#[function_component(SierpinskiComponent)]
fn sierpinski_component(props: &SierpinskiProps) -> Html {
    let points_list = props.sierpinski.vertices.chunks(3).map(|chunk| {
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

// HSL to RGB conversion function
fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r_prime, g_prime, b_prime) = if h >= 0.0 && h < 60.0 {
        (c, x, 0.0)
    } else if h >= 60.0 && h < 120.0 {
        (x, c, 0.0)
    } else if h >= 120.0 && h < 180.0 {
        (0.0, c, x)
    } else if h >= 180.0 && h < 240.0 {
        (0.0, x, c)
    } else if h >= 240.0 && h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    (((r_prime + m) * 255.0) as u8,
    ((g_prime + m) * 255.0) as u8,
    ((b_prime + m) * 255.0) as u8)
}

#[derive(Properties, PartialEq)]
pub struct MandelbrotProps {
    pub mandelbrot: Mandelbrot,
}

#[function_component(MandelbrotComponent)]
fn mandelbrot_component(props: &MandelbrotProps) -> Html {
    let node_ref = use_node_ref();

    {
        let mandelbrot = props.mandelbrot.clone();
        let node_ref = node_ref.clone();
        use_effect_with((mandelbrot.clone(),), move |_| {
            let canvas = node_ref.cast::<web_sys::HtmlCanvasElement>().unwrap();
            let context = canvas
                .get_context("2d")
                .unwrap()
                .unwrap()
                .dyn_into::<web_sys::CanvasRenderingContext2d>()
                .unwrap();

            let width = mandelbrot.width;
            let height = mandelbrot.height;
            canvas.set_width(width as u32);
            canvas.set_height(height as u32);
            let mut image_data_vec = vec![0u8; (width * height * 4) as usize];
            for i in 0..(width * height) {
                let iteration = mandelbrot.data[i];
                let color = if iteration == mandelbrot.max_iterations {
                    (0, 0, 0, 255) // Black for points in the set
                } else {
                    let hue = (iteration as f64 * 10.0) % 360.0;
                    let (r, g, b) = hsl_to_rgb(hue, 1.0, 0.5);
                    (r, g, b, 255)
                };
                let offset = i * 4;
                image_data_vec[offset] = color.0;
                image_data_vec[offset + 1] = color.1;
                image_data_vec[offset + 2] = color.2;
                image_data_vec[offset + 3] = color.3;
            }
            let image_data = web_sys::ImageData::new_with_u8_clamped_array_and_sh(
                Clamped(&mut image_data_vec),
                width as u32,
                height as u32,
            ).unwrap();
            context.put_image_data(&image_data, 0.0, 0.0).unwrap();

            || ()
        });
    }

    html! {
        <div class="fractal-container">
            <canvas ref={node_ref}></canvas>
        </div>
    }
}

#[derive(Properties, PartialEq)]
pub struct JuliaProps {
    pub julia: Julia,
}

#[function_component(JuliaComponent)]
fn julia_component(props: &JuliaProps) -> Html {
    let node_ref = use_node_ref();

    {
        let julia = props.julia.clone();
        let node_ref = node_ref.clone();
        use_effect_with((julia.clone(),), move |_| {
            let canvas = node_ref.cast::<web_sys::HtmlCanvasElement>().unwrap();
            let context = canvas
                .get_context("2d")
                .unwrap()
                .unwrap()
                .dyn_into::<web_sys::CanvasRenderingContext2d>()
                .unwrap();

            let width = julia.width;
            let height = julia.height;
            canvas.set_width(width as u32);
            canvas.set_height(height as u32);
            let mut image_data_vec = vec![0u8; (width * height * 4) as usize];
            for i in 0..(width * height) {
                let iteration = julia.data[i];
                let color = if iteration == julia.max_iterations {
                    (0, 0, 0, 255) // Black for points in the set
                } else {
                    let hue = (iteration as f64 * 10.0) % 360.0;
                    let (r, g, b) = hsl_to_rgb(hue, 1.0, 0.5);
                    (r, g, b, 255)
                };
                let offset = i * 4;
                image_data_vec[offset] = color.0;
                image_data_vec[offset + 1] = color.1;
                image_data_vec[offset + 2] = color.2;
                image_data_vec[offset + 3] = color.3;
            }
            let image_data = web_sys::ImageData::new_with_u8_clamped_array_and_sh(
                Clamped(&mut image_data_vec),
                width as u32,
                height as u32,
            ).unwrap();
            context.put_image_data(&image_data, 0.0, 0.0).unwrap();

            || ()
        });
    }

    html! {
        <div class="fractal-container">
            <canvas ref={node_ref}></canvas>
        </div>
    }
}


/// Properties for the `FractalComponent`.
#[derive(Properties, PartialEq)]
pub struct FractalProps {
    pub fractal: FractalData,
}

/// A Yew component for rendering a `FractalData` enum.
#[function_component(FractalComponent)]
fn fractal_component(props: &FractalProps) -> Html {
    match &props.fractal {
        FractalData::Sierpinski(s) => html! { <SierpinskiComponent sierpinski={s.clone()} /> },
        FractalData::Mandelbrot(m) => html! { <MandelbrotComponent mandelbrot={m.clone()} /> },
        FractalData::Julia(j) => html! { <JuliaComponent julia={j.clone()} /> },
    }
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(tag = "type", content = "params")]
pub enum MineRequestParams {
    Sierpinski {
        depth: usize,
    },
    Mandelbrot {
        width: usize,
        height: usize,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        max_iterations: u32,
    },
    Julia {
        width: usize,
        height: usize,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        c_real: f64,
        c_imag: f64,
        max_iterations: u32,
    },
}

#[function_component(MiningComponent)]
fn mining_component() -> Html {
    let fractal_type = use_state(|| "Sierpinski".to_string());
    let sierpinski_depth = use_state(|| 5);
    let mandelbrot_width = use_state(|| 50);
    let mandelbrot_height = use_state(|| 50);
    let mandelbrot_max_iter = use_state(|| 30);
    let julia_c_real = use_state(|| -0.8);
    let julia_c_imag = use_state(|| 0.156);
    let julia_width = use_state(|| 50);
    let julia_height = use_state(|| 50);
    let julia_max_iter = use_state(|| 100);

    let on_fractal_type_change = {
        let fractal_type = fractal_type.clone();
        Callback::from(move |e: Event| {
            let value = e.target_unchecked_into::<web_sys::HtmlSelectElement>().value();
            fractal_type.set(value);
        })
    };

    let on_mine_click = {
        let fractal_type = fractal_type.clone();
        let sierpinski_depth = sierpinski_depth.clone();
        let mandelbrot_width = mandelbrot_width.clone();
        let mandelbrot_height = mandelbrot_height.clone();
        let mandelbrot_max_iter = mandelbrot_max_iter.clone();
        let julia_c_real = julia_c_real.clone();
        let julia_c_imag = julia_c_imag.clone();
        let julia_width = julia_width.clone();
        let julia_height = julia_height.clone();
        let julia_max_iter = julia_max_iter.clone();

        Callback::from(move |_| {
            let params = match (*fractal_type).as_str() {
                "Sierpinski" => MineRequestParams::Sierpinski {
                    depth: *sierpinski_depth,
                },
                "Mandelbrot" => MineRequestParams::Mandelbrot {
                    width: *mandelbrot_width,
                    height: *mandelbrot_height,
                    x_min: -2.0, x_max: 1.0,
                    y_min: -1.5, y_max: 1.5,
                    max_iterations: *mandelbrot_max_iter,
                },
                "Julia" => MineRequestParams::Julia {
                    width: *julia_width,
                    height: *julia_height,
                    x_min: -1.5, x_max: 1.5,
                    y_min: -1.5, y_max: 1.5,
                    c_real: *julia_c_real,
                    c_imag: *julia_c_imag,
                    max_iterations: *julia_max_iter,
                },
                _ => unreachable!(),
            };
            spawn_local(async move {
                if let Ok(response) = Request::post("http://127.0.0.1:8081/mine")
                    .json(&params)
                    .unwrap()
                    .send()
                    .await
                {
                    if !response.ok() {
                        log::error!("Failed to mine block");
                    }
                }
            });
        })
    };

    html! {
        <div class="mining-card">
            <h2>{ "Mine a New Block" }</h2>
            <div>
                <label for="fractal_type">{ "Fractal Type:" }</label>
                <select id="fractal_type" onchange={on_fractal_type_change}>
                    <option value="Sierpinski" selected={*fractal_type == "Sierpinski"}>{ "Sierpinski" }</option>
                    <option value="Mandelbrot" selected={*fractal_type == "Mandelbrot"}>{ "Mandelbrot" }</option>
                    <option value="Julia" selected={*fractal_type == "Julia"}>{ "Julia" }</option>
                </select>
            </div>
            {
                match (*fractal_type).as_str() {
                    "Sierpinski" => html!{
                        <div>
                            <label for="sierpinski_depth">{ "Depth:" }</label>
                            <input type="number" id="sierpinski_depth" value={sierpinski_depth.to_string()} onchange={Callback::from(move |e: Event| {
                                let value = e.target_unchecked_into::<web_sys::HtmlInputElement>().value();
                                sierpinski_depth.set(value.parse().unwrap_or(5));
                            })} />
                        </div>
                    },
                    "Mandelbrot" => html!{
                        <>
                            <div>
                                <label for="mandelbrot_width">{ "Width:" }</label>
                                <input type="number" id="mandelbrot_width" value={mandelbrot_width.to_string()} onchange={Callback::from(move |e: Event| {
                                    let value = e.target_unchecked_into::<web_sys::HtmlInputElement>().value();
                                    mandelbrot_width.set(value.parse().unwrap_or(50));
                                })}/>
                            </div>
                            <div>
                                <label for="mandelbrot_height">{ "Height:" }</label>
                                <input type="number" id="mandelbrot_height" value={mandelbrot_height.to_string()} onchange={Callback::from(move |e: Event| {
                                    let value = e.target_unchecked_into::<web_sys::HtmlInputElement>().value();
                                    mandelbrot_height.set(value.parse().unwrap_or(50));
                                })}/>
                            </div>
                            <div>
                                <label for="mandelbrot_max_iter">{ "Max Iterations:" }</label>
                                <input type="number" id="mandelbrot_max_iter" value={mandelbrot_max_iter.to_string()} onchange={Callback::from(move |e: Event| {
                                    let value = e.target_unchecked_into::<web_sys::HtmlInputElement>().value();
                                    mandelbrot_max_iter.set(value.parse().unwrap_or(30));
                                })}/>
                            </div>
                        </>
                    },
                    "Julia" => html!{
                        <>
                            <div>
                                <label for="julia_width">{ "Width:" }</label>
                                <input type="number" id="julia_width" value={julia_width.to_string()} onchange={Callback::from(move |e: Event| {
                                    let value = e.target_unchecked_into::<web_sys::HtmlInputElement>().value();
                                    julia_width.set(value.parse().unwrap_or(50));
                                })}/>
                            </div>
                            <div>
                                <label for="julia_height">{ "Height:" }</label>
                                <input type="number" id="julia_height" value={julia_height.to_string()} onchange={Callback::from(move |e: Event| {
                                    let value = e.target_unchecked_into::<web_sys::HtmlInputElement>().value();
                                    julia_height.set(value.parse().unwrap_or(50));
                                })}/>
                            </div>
                            <div>
                                <label for="julia_max_iter">{ "Max Iterations:" }</label>
                                <input type="number" id="julia_max_iter" value={julia_max_iter.to_string()} onchange={Callback::from(move |e: Event| {
                                    let value = e.target_unchecked_into::<web_sys::HtmlInputElement>().value();
                                    julia_max_iter.set(value.parse().unwrap_or(100));
                                })}/>
                            </div>
                            <div>
                                <label for="julia_c_real">{ "C (Real):" }</label>
                                <input type="number" step="0.01" id="julia_c_real" value={julia_c_real.to_string()} onchange={Callback::from(move |e: Event| {
                                    let value = e.target_unchecked_into::<web_sys::HtmlInputElement>().value();
                                    julia_c_real.set(value.parse().unwrap_or(-0.8));
                                })}/>
                            </div>
                            <div>
                                <label for="julia_c_imag">{ "C (Imaginary):" }</label>
                                <input type="number" step="0.001" id="julia_c_imag" value={julia_c_imag.to_string()} onchange={Callback::from(move |e: Event| {
                                    let value = e.target_unchecked_into::<web_sys::HtmlInputElement>().value();
                                    julia_c_imag.set(value.parse().unwrap_or(0.156));
                                })}/>
                            </div>
                        </>
                    },
                    _ => html! {}
                }
            }
            <button onclick={on_mine_click}>{ "Mine Block" }</button>
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
                if let Ok(response) = Request::get("http://127.0.0.1:8081/wallet/info").send().await {
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
                if let Ok(response) = Request::post("http://127.0.0.1:8081/transact").json(&req).unwrap().send().await {
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
                if let Ok(response) = Request::get("http://127.0.0.1:8081/blocks").send().await {
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
            let ws_conn = WebSocket::open("ws://127.0.0.1:8081/ws").unwrap();
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
                    <MiningComponent />
                    <WalletComponent />
                </div>
                <div class="main-content">
                    if blocks.is_empty() {
                        <p>{ "Loading blocks..." }</p>
                    } else {
                        <div class="blocks-container">
                            { for blocks.iter().rev().map(|block| html! {
                                <div class="block-card">
                                    <FractalComponent fractal={block.fractal.clone()} />
                                    <div class="block-details">
                                        <h2>{ format!("Block #{}", block.index) }</h2>
                                        <p><strong>{ "Hash: " }</strong>{ &block.hash }</p>
                                        <p><strong>{ "Prev. Hash: " }</strong>{ &block.previous_hash }</p>
                                        <p><strong>{ "Nonce: " }</strong>{ block.nonce }</p>
                                        <p><strong>{ "Transactions: " }</strong>{ block.transactions.len() }</p>
                                        {
                                            match &block.fractal {
                                                FractalData::Sierpinski(s) => html!{<p><strong>{ "Fractal Type: " }</strong>{ "Sierpinski" }<br/><strong>{ "Depth: " }</strong>{ s.depth }</p>},
                                                FractalData::Mandelbrot(m) => html!{<p><strong>{ "Fractal Type: " }</strong>{ "Mandelbrot" }<br/><strong>{ "Max Iterations: " }</strong>{ m.max_iterations }</p>},
                                                FractalData::Julia(j) => html!{<p><strong>{ "Fractal Type: " }</strong>{ "Julia" }<br/><strong>{ "Max Iterations: " }</strong>{ j.max_iterations }<br/><strong>{ "C: " }</strong>{ format!("{:.3} + {:.3}i", j.c_real, j.c_imag) }</p>},
                                            }
                                        }
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

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn pass() {
        assert_eq!(1, 1);
    }
}
