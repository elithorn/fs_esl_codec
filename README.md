fs_esl_codec
---

This is a freeswitch esl codec for parsing event stream from freeswitch in inbound mode.
This crate only provides a codec. The authentication and the initial request for events in the appropriate format is to be done separately (see examples/untyped.rs for the most basic one).
The framing mechanism returns a packet consisting of ESL headers (not to be confused with the actual event headers) such as content-type, and a String buffer, that you can deserialize however you want using the parser appropriate for the type of events requested (events plain/events json/events xml).
This way you can put this codec on a reader of a .split(), or onto an another socket entirely.

