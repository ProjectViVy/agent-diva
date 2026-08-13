# Summary

Added a "More Providers" fold in the GUI Settings -> Providers page to hide less commonly used providers by default.

The following 20 providers are now folded:
302.AI, AIhubMix, AIonly, Baichuan, BurnCloud, Cephalon, Cerebras, CherryIN, Fireworks AI, Hyperbolic, Infini/Maas, Jina AI, Lanyun, OcoolAI, PH8, PPIO, Together AI, TokenFlux, Voyage AI, Yi.

Baidu Qianfan remains in the common/visible list.

The default list is shorter and easier to scan. Hidden providers are still reachable by expanding the "More Providers" row, and search overrides the fold so typing a provider name still finds it immediately. A reusable `ProviderListItem.vue` component was extracted so the visible and folded lists share the same rendering logic.
