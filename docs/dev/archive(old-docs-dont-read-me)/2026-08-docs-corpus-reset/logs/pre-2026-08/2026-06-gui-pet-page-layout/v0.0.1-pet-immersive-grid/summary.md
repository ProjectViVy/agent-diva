# Summary

Fixed the GUI pet page opened from the sidebar becoming invisible in immersive mode.

The pet route removes the regular sidebar, so the shell must use a single visible grid column. The previous immersive CSS left the main panel in a zero-width grid track, hiding the pet page.

