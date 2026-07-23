# Frozen protocol v1 evidence

These fixtures preserve the exact lockstep envelopes and the first real
DevPULSE shell flow that existed before protocol v2. They are compatibility
evidence only: production runtime and runner code contain no v1 bridge.

`v1_compat.rs` reconstructs every JSON example with the original serde shape
and verifies that the durable shell scenario is unchanged.
