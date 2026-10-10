# Development acceptance

Use a synthetic, isolated profile. Enable reflection while a foreground response is held: automatic ticks must not invoke inference or advance the watermark. Release that real response: every original input must be covered by contiguous windows, with no later self-trigger.

Set a 20-second minimum interval after the first settled input, then admit a second input. Its native capture becomes ready before inference; inference stays deferred and later runs once using the real clock.

While reflection is active, issue twelve concurrent trigger actions and admit another human turn. All triggers return active and the same Run identity; the new turn remains pending until the original window settles.

Inspect native ingestion and durable status after reflection: derived workflow/child/supervisor output and reflection notes must not become human capture sources. These probes do not replace the complete formal matrix or Windows/live-model acceptance.
