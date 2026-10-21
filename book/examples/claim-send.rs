//! Claim, encode, check, commit: the generated `send` / `send_fixed` over
//! `sbe_rt::Claim`. Compiled against the feature-tour codec by the
//! book-fence test; `OfferClaim` stands in for a transport.

// ANCHOR: send_fixed
// Offer style: encode into `scratch`; commit hands the frame to the transport.
let mut scratch = [0u8; HeartbeatEncoder::ENCODED_LENGTH];
let mut published: Vec<Vec<u8>> = Vec::new();
let claim = sbe_rt::OfferClaim::new(&mut scratch, |frame: &[u8]| -> Result<(), &'static str> {
    published.push(frame.to_vec()); // publication.offer(frame).map(drop)
    Ok(())
});
HeartbeatEncoder::send_fixed(claim, &HeartbeatFixedFields { sequence: 7, timestamp: 0 })?;
assert_eq!(published.len(), 1);
// ANCHOR_END: send_fixed

// ANCHOR: send
let body = b"rebalance at close";
let len = NoteEncoder::compute_length_with_header(body.len());
let mut scratch = vec![0u8; len];
let claim = sbe_rt::OfferClaim::new(&mut scratch, |frame: &[u8]| -> Result<(), &'static str> {
    assert_eq!(frame.len(), len);
    Ok(())
});
// The closure gets the encoder wrapped on the claim and must return the
// finished frame: the type-state makes a half-written one a compile error.
NoteEncoder::send(claim, |enc| enc.fixed(&NoteFixedFields { note_id: 1 }).body(body))?;
// ANCHOR_END: send

// ANCHOR: length_mismatch
// A claim one byte too long: nothing is offered, and the error says why.
let body = b"rebalance at close";
let len = NoteEncoder::compute_length_with_header(body.len());
let mut too_long = vec![0u8; len + 1];
let claim = sbe_rt::OfferClaim::new(&mut too_long, |_: &[u8]| -> Result<(), &'static str> {
    unreachable!("an aborted claim is never offered")
});
let sent = NoteEncoder::send(claim, |enc| enc.fixed(&NoteFixedFields { note_id: 1 }).body(body));
assert_eq!(
    sent,
    Err(sbe_rt::SendError::LengthMismatch { claimed: len + 1, frame: len })
);
// ANCHOR_END: length_mismatch

// ANCHOR: source_trait
/// The application's own type: no generated struct is involved.
struct Reminder {
    id: u32,
    text: String,
}

impl NoteSource for Reminder {
    fn note_id(&self) -> u32 {
        self.id
    }
    fn body(&self) -> &[u8] {
        self.text.as_bytes()
    }
}

let reminder = Reminder { id: 9, text: "settle T+1".into() };
let len = NoteEncoder::encoded_length_with_header_from(&reminder)?;
let mut scratch = vec![0u8; len];
let claim = sbe_rt::OfferClaim::new(&mut scratch, |_: &[u8]| -> Result<(), &'static str> { Ok(()) });
NoteEncoder::send(claim, |enc| enc.encode_from(&reminder))?;
// ANCHOR_END: source_trait
