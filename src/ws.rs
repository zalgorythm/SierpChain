use actix::prelude::*;
use actix_web_actors::ws;
use std::collections::HashMap;
use rand::{self, Rng, thread_rng};
use crate::block::Block;

/// Message with new block from miner to broadcaster
#[derive(Message)]
#[rtype(result = "()")]
pub struct BlockMessage(pub Block);

/// Message with JSON-encoded block from broadcaster to a connection
#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct TextMessage(pub String);

/// Message from a WsConn to the broadcaster to connect.
#[derive(Message)]
#[rtype(result = "usize")]
pub struct Connect {
    pub addr: Recipient<TextMessage>,
}

/// Message sent from a WsConn to the broadcaster to disconnect.
#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub id: usize,
}

/// The broadcaster actor.
#[derive(Default)]
pub struct Broadcaster {
    sessions: HashMap<usize, Recipient<TextMessage>>,
}

impl Actor for Broadcaster {
    type Context = Context<Self>;
}

impl Handler<Connect> for Broadcaster {
    type Result = usize;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        let id = thread_rng().r#gen::<usize>();
        self.sessions.insert(id, msg.addr);
        id
    }
}

impl Handler<Disconnect> for Broadcaster {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        self.sessions.remove(&msg.id);
    }
}

impl Handler<BlockMessage> for Broadcaster {
    type Result = ();

    fn handle(&mut self, msg: BlockMessage, _: &mut Context<Self>) {
        let block_json = serde_json::to_string(&msg.0).unwrap();
        let text_message = TextMessage(block_json);
        for addr in self.sessions.values() {
            let _ = addr.do_send(text_message.clone());
        }
    }
}


/// The WebSocket connection actor.
pub struct WsConn {
    id: usize,
    broadcaster_addr: Addr<Broadcaster>,
}

impl WsConn {
    pub fn new(broadcaster_addr: Addr<Broadcaster>) -> WsConn {
        WsConn {
            id: 0, // Will be set when connected to the broadcaster
            broadcaster_addr,
        }
    }
}

impl Actor for WsConn {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let addr = ctx.address().recipient();
        self.broadcaster_addr
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
        self.broadcaster_addr.do_send(Disconnect { id: self.id });
        Running::Stop
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsConn {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => ctx.text(text), // Echo back text
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => (),
        }
    }
}

impl Handler<TextMessage> for WsConn {
    type Result = ();

    fn handle(&mut self, msg: TextMessage, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}
