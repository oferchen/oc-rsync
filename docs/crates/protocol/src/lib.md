# crates/protocol/src/lib.rs

Core frame and message types for the rsync protocol.


Messages are exchanged in several phases including version negotiation,

file-list transmission, attribute exchange, progress updates and error

reporting. Each phase corresponds to a [`Message`] variant encoded inside

multiplexed [`Frame`] structures.
