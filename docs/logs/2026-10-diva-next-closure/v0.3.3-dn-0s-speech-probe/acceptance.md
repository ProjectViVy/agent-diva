# v0.3.3 — DN-0S acceptance

Owner acceptance pending. To verify:

1. Read `docs/plans/diva-next/fixtures/closure-speech.json` — every seam must
   carry either evidence or an explicit pending marker; nothing may be
   silently claimed.
2. Confirm the C2-4 amendment pins `keyring 4.2.0` store features and the
   `reqwest 0.13` rustls feature set in the ledger.
3. Spot-check that downstream gates (Windows WebView2 roundtrip, Credential
   Manager, authenticated provider calls) are listed as pending — DN-8C/DN-6
   own them.
4. Probes are disposable: `~/staging/dn-0s/` is not part of the repo; nothing
   in product code changed.
