// negative fixture: restored pet_* business invokes outside the dormant
// sealed feature are denied — activated speech lives in desktop-host.
export const pet = () => (globalThis as any).invoke('pet_feed', {})
