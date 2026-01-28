//! MSP Text Profile - profile:textual implementation
//!
//! Text state atoms:
//! - ω:text:stream_id - Chat message stream
//! - ω:presence:user_id - User presence
//! - ω:typing:user_id - Typing indicator

use elara_core::{
    DeltaLaw, MessageId, NodeId, StateAtom, StateBounds, StateId, StateTime, StateType,
};

/// State type prefixes for text profile
pub const STATE_TYPE_TEXT: u16 = 0x0001;
pub const STATE_TYPE_PRESENCE: u16 = 0x0002;
pub const STATE_TYPE_TYPING: u16 = 0x0003;
pub const STATE_TYPE_FEED: u16 = 0x0004;

/// Create a text stream state ID
pub fn text_stream_id(stream_id: u64) -> StateId {
    StateId::from_type_instance(STATE_TYPE_TEXT, stream_id)
}

/// Create a presence state ID
pub fn presence_id(user_id: NodeId) -> StateId {
    StateId::from_type_instance(STATE_TYPE_PRESENCE, user_id.0)
}

/// Create a typing state ID
pub fn typing_id(user_id: NodeId) -> StateId {
    StateId::from_type_instance(STATE_TYPE_TYPING, user_id.0)
}

pub fn feed_stream_id(stream_id: u64) -> StateId {
    StateId::from_type_instance(STATE_TYPE_FEED, stream_id)
}

/// Text message in a stream
#[derive(Clone, Debug)]
pub struct TextMessage {
    /// Unique message ID
    pub id: MessageId,
    /// Author node ID
    pub author: NodeId,
    /// Message content (encrypted in transit)
    pub content: Vec<u8>,
    /// Timestamp
    pub timestamp: StateTime,
    /// Reply to another message (optional)
    pub reply_to: Option<MessageId>,
    /// Edit of another message (optional)
    pub edit_of: Option<MessageId>,
    /// Is this message deleted (soft delete)
    pub deleted: bool,
}

#[derive(Clone, Debug)]
pub struct FeedItem {
    pub id: MessageId,
    pub author: NodeId,
    pub content: Vec<u8>,
    pub timestamp: StateTime,
    pub deleted: bool,
}

impl FeedItem {
    pub fn new(id: MessageId, author: NodeId, content: Vec<u8>, timestamp: StateTime) -> Self {
        FeedItem {
            id,
            author,
            content,
            timestamp,
            deleted: false,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.id.0.to_le_bytes());
        buf.extend_from_slice(&self.author.to_bytes());
        buf.extend_from_slice(&self.timestamp.as_micros().to_le_bytes());
        buf.push(if self.deleted { 1 } else { 0 });
        buf.extend_from_slice(&(self.content.len() as u16).to_le_bytes());
        buf.extend_from_slice(&self.content);
        buf
    }

    pub fn decode(buf: &[u8]) -> Option<(Self, usize)> {
        if buf.len() < 27 {
            return None;
        }

        let id = MessageId(u64::from_le_bytes(buf[0..8].try_into().ok()?));
        let author = NodeId::from_bytes(buf[8..16].try_into().ok()?);
        let timestamp = StateTime::from_micros(i64::from_le_bytes(buf[16..24].try_into().ok()?));
        let deleted = buf[24] != 0;
        let mut offset = 25;

        if buf.len() < offset + 2 {
            return None;
        }
        let content_len = u16::from_le_bytes(buf[offset..offset + 2].try_into().ok()?) as usize;
        offset += 2;

        if buf.len() < offset + content_len {
            return None;
        }
        let content = buf[offset..offset + content_len].to_vec();
        offset += content_len;

        Some((
            FeedItem {
                id,
                author,
                content,
                timestamp,
                deleted,
            },
            offset,
        ))
    }
}

impl TextMessage {
    pub fn new(id: MessageId, author: NodeId, content: Vec<u8>, timestamp: StateTime) -> Self {
        TextMessage {
            id,
            author,
            content,
            timestamp,
            reply_to: None,
            edit_of: None,
            deleted: false,
        }
    }

    /// Encode message for wire format
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        // Message ID (8 bytes)
        buf.extend_from_slice(&self.id.0.to_le_bytes());

        // Author (8 bytes)
        buf.extend_from_slice(&self.author.to_bytes());

        // Timestamp (8 bytes)
        buf.extend_from_slice(&self.timestamp.as_micros().to_le_bytes());

        // Flags (1 byte)
        let mut flags = 0u8;
        if self.reply_to.is_some() {
            flags |= 0x01;
        }
        if self.edit_of.is_some() {
            flags |= 0x02;
        }
        if self.deleted {
            flags |= 0x04;
        }
        buf.push(flags);

        // Optional reply_to (8 bytes if present)
        if let Some(reply_to) = self.reply_to {
            buf.extend_from_slice(&reply_to.0.to_le_bytes());
        }

        // Optional edit_of (8 bytes if present)
        if let Some(edit_of) = self.edit_of {
            buf.extend_from_slice(&edit_of.0.to_le_bytes());
        }

        // Content length (2 bytes) + content
        buf.extend_from_slice(&(self.content.len() as u16).to_le_bytes());
        buf.extend_from_slice(&self.content);

        buf
    }

    /// Decode message from wire format
    pub fn decode(buf: &[u8]) -> Option<(Self, usize)> {
        if buf.len() < 25 {
            return None;
        }

        let id = MessageId(u64::from_le_bytes(buf[0..8].try_into().ok()?));
        let author = NodeId::from_bytes(buf[8..16].try_into().ok()?);
        let timestamp = StateTime::from_micros(i64::from_le_bytes(buf[16..24].try_into().ok()?));
        let flags = buf[24];

        let mut offset = 25;

        let reply_to = if flags & 0x01 != 0 {
            if buf.len() < offset + 8 {
                return None;
            }
            let id = MessageId(u64::from_le_bytes(buf[offset..offset + 8].try_into().ok()?));
            offset += 8;
            Some(id)
        } else {
            None
        };

        let edit_of = if flags & 0x02 != 0 {
            if buf.len() < offset + 8 {
                return None;
            }
            let id = MessageId(u64::from_le_bytes(buf[offset..offset + 8].try_into().ok()?));
            offset += 8;
            Some(id)
        } else {
            None
        };

        let deleted = flags & 0x04 != 0;

        if buf.len() < offset + 2 {
            return None;
        }
        let content_len = u16::from_le_bytes(buf[offset..offset + 2].try_into().ok()?) as usize;
        offset += 2;

        if buf.len() < offset + content_len {
            return None;
        }
        let content = buf[offset..offset + content_len].to_vec();
        offset += content_len;

        Some((
            TextMessage {
                id,
                author,
                content,
                timestamp,
                reply_to,
                edit_of,
                deleted,
            },
            offset,
        ))
    }
}

/// Text stream state - a collection of messages
#[derive(Clone, Debug, Default)]
pub struct TextStream {
    /// Messages in causal order
    pub messages: Vec<TextMessage>,
    /// Maximum messages to keep
    pub max_messages: usize,
}

#[derive(Clone, Debug, Default)]
pub struct FeedStream {
    pub items: Vec<FeedItem>,
    pub max_items: usize,
}

impl FeedStream {
    pub fn new(max_items: usize) -> Self {
        FeedStream {
            items: Vec::new(),
            max_items,
        }
    }

    pub fn append(&mut self, item: FeedItem) {
        if self.items.iter().any(|m| m.id == item.id) {
            return;
        }

        let pos = self
            .items
            .binary_search_by(|m| m.timestamp.cmp(&item.timestamp))
            .unwrap_or_else(|p| p);

        self.items.insert(pos, item);

        if self.items.len() > self.max_items {
            self.items.remove(0);
        }
    }

    pub fn delete(&mut self, item_id: MessageId) -> bool {
        if let Some(item) = self.items.iter_mut().find(|m| m.id == item_id) {
            item.deleted = true;
            item.content.clear();
            true
        } else {
            false
        }
    }

    pub fn recent(&self, count: usize) -> &[FeedItem] {
        let start = self.items.len().saturating_sub(count);
        &self.items[start..]
    }
}

impl TextStream {
    pub fn new(max_messages: usize) -> Self {
        TextStream {
            messages: Vec::new(),
            max_messages,
        }
    }

    /// Append a message (CRDT append-only)
    pub fn append(&mut self, message: TextMessage) {
        // Check for duplicate
        if self.messages.iter().any(|m| m.id == message.id) {
            return;
        }

        // Insert in timestamp order
        let pos = self
            .messages
            .binary_search_by(|m| m.timestamp.cmp(&message.timestamp))
            .unwrap_or_else(|p| p);

        self.messages.insert(pos, message);

        // Trim if needed
        if self.messages.len() > self.max_messages {
            self.messages.remove(0);
        }
    }

    /// Edit a message
    pub fn edit(&mut self, message_id: MessageId, new_content: Vec<u8>) -> bool {
        if let Some(msg) = self.messages.iter_mut().find(|m| m.id == message_id) {
            msg.content = new_content;
            true
        } else {
            false
        }
    }

    /// Delete a message (soft delete)
    pub fn delete(&mut self, message_id: MessageId) -> bool {
        if let Some(msg) = self.messages.iter_mut().find(|m| m.id == message_id) {
            msg.deleted = true;
            msg.content.clear();
            true
        } else {
            false
        }
    }

    /// Get message by ID
    pub fn get(&self, message_id: MessageId) -> Option<&TextMessage> {
        self.messages.iter().find(|m| m.id == message_id)
    }

    /// Get recent messages
    pub fn recent(&self, count: usize) -> &[TextMessage] {
        let start = self.messages.len().saturating_sub(count);
        &self.messages[start..]
    }

    /// Encode stream for wire format
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        // Message count (2 bytes)
        buf.extend_from_slice(&(self.messages.len() as u16).to_le_bytes());

        // Messages
        for msg in &self.messages {
            let encoded = msg.encode();
            buf.extend_from_slice(&(encoded.len() as u16).to_le_bytes());
            buf.extend_from_slice(&encoded);
        }

        buf
    }
}

/// Presence status
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum PresenceStatus {
    Online = 0,
    Away = 1,
    Busy = 2,
    Offline = 3,
}

impl PresenceStatus {
    pub fn from_byte(b: u8) -> Self {
        match b {
            0 => PresenceStatus::Online,
            1 => PresenceStatus::Away,
            2 => PresenceStatus::Busy,
            _ => PresenceStatus::Offline,
        }
    }
}

/// Presence state
#[derive(Clone, Debug)]
pub struct Presence {
    pub user_id: NodeId,
    pub status: PresenceStatus,
    pub last_active: StateTime,
    pub custom_status: Option<String>,
}

impl Presence {
    pub fn new(user_id: NodeId) -> Self {
        Presence {
            user_id,
            status: PresenceStatus::Online,
            last_active: StateTime::ZERO,
            custom_status: None,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.user_id.to_bytes());
        buf.push(self.status as u8);
        buf.extend_from_slice(&self.last_active.as_micros().to_le_bytes());

        if let Some(ref status) = self.custom_status {
            buf.push(1);
            buf.extend_from_slice(&(status.len() as u16).to_le_bytes());
            buf.extend_from_slice(status.as_bytes());
        } else {
            buf.push(0);
        }

        buf
    }
}

/// Typing indicator state
#[derive(Clone, Debug)]
pub struct TypingIndicator {
    pub user_id: NodeId,
    pub is_typing: bool,
    pub started_at: StateTime,
    /// Auto-expire after this duration (typically 5 seconds)
    pub ttl_ms: u32,
}

impl TypingIndicator {
    pub fn new(user_id: NodeId) -> Self {
        TypingIndicator {
            user_id,
            is_typing: false,
            started_at: StateTime::ZERO,
            ttl_ms: 5000,
        }
    }

    pub fn start_typing(&mut self, now: StateTime) {
        self.is_typing = true;
        self.started_at = now;
    }

    pub fn stop_typing(&mut self) {
        self.is_typing = false;
    }

    pub fn is_expired(&self, now: StateTime) -> bool {
        if !self.is_typing {
            return true;
        }
        let elapsed = now.as_millis() - self.started_at.as_millis();
        elapsed > self.ttl_ms as i64
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.user_id.to_bytes());
        buf.push(if self.is_typing { 1 } else { 0 });
        buf.extend_from_slice(&self.started_at.as_micros().to_le_bytes());
        buf.extend_from_slice(&self.ttl_ms.to_le_bytes());
        buf
    }
}

/// Create a text state atom
pub fn create_text_atom(stream_id: u64, owner: NodeId) -> StateAtom {
    let mut atom = StateAtom::new(text_stream_id(stream_id), StateType::Core, owner);
    atom.delta_law = DeltaLaw::AppendOnly { max_size: 1000 };
    atom.bounds = StateBounds {
        max_size: 1024 * 1024,                                  // 1MB
        rate_limit: Some(elara_core::RateLimit::new(10, 1000)), // 10 msg/sec
        max_entropy: 1.0,
    };
    atom
}

pub fn create_feed_atom(stream_id: u64, owner: NodeId) -> StateAtom {
    let mut atom = StateAtom::new(feed_stream_id(stream_id), StateType::Core, owner);
    atom.delta_law = DeltaLaw::AppendOnly { max_size: 5000 };
    atom.bounds = StateBounds {
        max_size: 5 * 1024 * 1024,
        rate_limit: Some(elara_core::RateLimit::new(5, 1000)),
        max_entropy: 1.0,
    };
    atom
}

/// Create a presence state atom
pub fn create_presence_atom(user_id: NodeId) -> StateAtom {
    let mut atom = StateAtom::new(presence_id(user_id), StateType::Core, user_id);
    atom.delta_law = DeltaLaw::LastWriterWins;
    atom
}

/// Create a typing state atom
pub fn create_typing_atom(user_id: NodeId) -> StateAtom {
    let mut atom = StateAtom::new(typing_id(user_id), StateType::Perceptual, user_id);
    atom.delta_law = DeltaLaw::LastWriterWins;
    atom
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_message_roundtrip() {
        let msg = TextMessage::new(
            MessageId::new(12345),
            NodeId::new(1),
            b"Hello, ELARA!".to_vec(),
            StateTime::from_millis(1000),
        );

        let encoded = msg.encode();
        let (decoded, _) = TextMessage::decode(&encoded).unwrap();

        assert_eq!(decoded.id, msg.id);
        assert_eq!(decoded.author, msg.author);
        assert_eq!(decoded.content, msg.content);
    }

    #[test]
    fn test_text_stream_append() {
        let mut stream = TextStream::new(100);

        let msg1 = TextMessage::new(
            MessageId::new(1),
            NodeId::new(1),
            b"First".to_vec(),
            StateTime::from_millis(1000),
        );

        let msg2 = TextMessage::new(
            MessageId::new(2),
            NodeId::new(1),
            b"Second".to_vec(),
            StateTime::from_millis(2000),
        );

        stream.append(msg1);
        stream.append(msg2);

        assert_eq!(stream.messages.len(), 2);

        // Duplicate should be ignored
        let msg1_dup = TextMessage::new(
            MessageId::new(1),
            NodeId::new(1),
            b"Duplicate".to_vec(),
            StateTime::from_millis(1000),
        );
        stream.append(msg1_dup);

        assert_eq!(stream.messages.len(), 2);
    }

    #[test]
    fn test_feed_item_roundtrip() {
        let item = FeedItem::new(
            MessageId::new(44),
            NodeId::new(7),
            b"feed".to_vec(),
            StateTime::from_millis(4000),
        );

        let encoded = item.encode();
        let (decoded, _) = FeedItem::decode(&encoded).unwrap();

        assert_eq!(decoded.id, item.id);
        assert_eq!(decoded.author, item.author);
        assert_eq!(decoded.content, item.content);
    }

    #[test]
    fn test_feed_stream_append() {
        let mut stream = FeedStream::new(10);

        let item1 = FeedItem::new(
            MessageId::new(1),
            NodeId::new(1),
            b"a".to_vec(),
            StateTime::from_millis(1000),
        );

        let item2 = FeedItem::new(
            MessageId::new(2),
            NodeId::new(1),
            b"b".to_vec(),
            StateTime::from_millis(2000),
        );

        stream.append(item1);
        stream.append(item2);

        assert_eq!(stream.items.len(), 2);
    }

    #[test]
    fn test_typing_indicator_expiry() {
        let mut typing = TypingIndicator::new(NodeId::new(1));
        typing.ttl_ms = 5000;

        typing.start_typing(StateTime::from_millis(1000));
        assert!(!typing.is_expired(StateTime::from_millis(3000)));
        assert!(typing.is_expired(StateTime::from_millis(7000)));
    }
}
