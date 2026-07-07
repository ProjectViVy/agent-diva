# Summary

Added a "More Providers" fold in the GUI Settings -> Providers page to hide less commonly used providers by default.

The following 19 providers are now folded:
302.AI, AIonly, Baichuan, Baidu Qianfan, BurnCloud, Cephalon, Cerebras, Fireworks AI, Hyperbolic, Infini/Maas, Jina AI, Lanyun, OcoolAI, PH8, PPIO, Together AI, TokenFlux, Voyage AI, Yi.

The default list is shorter and easier to scan. Hidden providers are still reachable by expanding the "More Providers" row, and search overrides the fold so typing a provider name still finds it immediately. A reusable `ProviderListItem.vue` component was extracted so the visible and folded lists share the same rendering logic.
