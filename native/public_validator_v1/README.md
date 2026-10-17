# Synthetic public-validator v1 vectors

See `PUBLIC_VALIDATOR_V1.md` for the authoritative schema, limits, trust boundary
and reproduction targets. Fixtures contain synthetic public MLS data only, not
private keys or private storage. Manifest hashes bind the complete exact frames.

Success cases: current snapshot, stable-credential/fresh-key external successor,
its consecutive replacement, and a multi-leaf replacement. Failures: unauthorized
new signature key (23), absent authorized old leaf (24), empty AAD (6), invalid
GroupInfo signature (31), retained-member credential/key collision (24), and
reuse of the authorized removed leaf's old key (24). Every failure has a null
result and no partial state. There are eleven request/response pairs.

The public validator cannot validate secret-dependent confirmation MACs. These
vectors do not establish account authorization, database atomicity or mobile
resource limits. The fixture generator does not run during ordinary tests.
