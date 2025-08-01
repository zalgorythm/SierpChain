use actix::{Actor, Addr, ActorContext, ActorFutureExt, AsyncContext, Context, ContextFutureSpawner, fut, Handler, Message, Recipient, Running, StreamHandler, WrapFuture};
use actix_web_actors::ws;
use std::collections::HashMap;
use crate::blockchain::block::Block;

/// Message sent from the `BroadcastHub` to a specific client.
#[derive(Message)]
#[rtype(result = "()")]
pub struct ClientMessage(pub String);

/// Message to broadcast a new block.
#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct BroadcastBlock {
    pub block: Block,
}

/// The central hub for broadcasting messages to all WebSocket clients.
#[derive(Default)]
pub struct BroadcastHub {
    sessions: HashMap<usize, Recipient<ClientMessage>>,
    next_id: usize,
}

impl BroadcastHub {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Actor for BroadcastHub {
    type Context = Context<Self>;
}

impl Handler<Connect> for BroadcastHub {
    type Result = usize;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        let id = self.next_id;
        self.sessions.insert(id, msg.addr);
        self.next_id += 1;
        id
    }
}

impl Handler<Disconnect> for BroadcastHub {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        self.sessions.remove(&msg.id);
    }
}

impl Handler<BroadcastBlock> for BroadcastHub {
    type Result = ();

    fn handle(&mut self, msg: BroadcastBlock, _: &mut Context<Self>) {
        let block_json = serde_json::to_string(&msg.block).unwrap();
        for addr in self.sessions.values() {
            addr.do_send(ClientMessage(block_json.clone()));
        }
    }
}

/// Message to connect a new WebSocket session to the `BroadcastHub`.
#[derive(Message)]
#[rtype(usize)]
pub struct Connect {
    pub addr: Recipient<ClientMessage>,
}

/// Message to disconnect a WebSocket session from the `BroadcastHub`.
#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub id: usize,
}

/// The WebSocket connection actor.
pub struct WsConn {
    id: usize,
    hub_addr: Addr<BroadcastHub>,
}

impl WsConn {
    pub fn new(hub_addr: Addr<BroadcastHub>) -> Self {
        Self { id: 0, hub_addr }
    }
}

impl Actor for WsConn {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let addr = ctx.address().recipient();
        self.hub_addr
            .send(Connect { addr })
            .into_actor(self)
            .then(|res, act, ctx| {
                match res {
                    Ok(id) => act.id = id,
                    _ => ctx.stop(),
                }
                fut::ready(())
            })
            .wait(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        self.hub_addr.do_send(Disconnect { id: self.id });
        Running::Stop
    }
}

impl Handler<ClientMessage> for WsConn {
    type Result = ();

    fn handle(&mut self, msg: ClientMessage, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsConn {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => (), // We don't expect messages from the client in this app
        }
    }
}
