# Onca skills (Tier 1)

ZeroClaw skills are `SKILL.md` capabilities the agent composes from built-in
tools — no compiled code. Correct layering: the read side of the oracle is a
skill over the built-in `http_request`, not a plugin.

## `settle-weather-market`

Teaches the agent to settle a live Solana weather prediction market on the Onca
mesh instead of a single source. It composes two tools it already has:

- `mesh_oracle` (the T0 plugin) for the trusted temperature, and
- the built-in `http_request` for the live market from Jupiter's keyless
  prediction API (Polymarket liquidity on Solana).

The agent reads the market, maps the mesh value to the winning outcome bucket,
and reports it — no single source in the loop.

### Install

```bash
zeroclaw skills bundle add onca                       # creates the bundle dir
cp -r settle-weather-market ~/.zeroclaw/shared/skills/onca/
```

Then wire it into the agent in `~/.zeroclaw/config.toml`:

```toml
[agents.onca]
skill_bundles = ["onca"]

[risk_profiles.onca]
allowed_tools = ["depin_attest", "mesh_oracle", "http_request"]
auto_approve  = [..., "http_request", "mesh_oracle"]   # reads need no tap
```

Restart the daemon, then in the channel: *"settle the São Paulo weather market
on our mesh."* The agent does the rest.
