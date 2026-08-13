# Summary

Fixed the provider list in GUI settings so clicking a provider other than the current one actually selects it.

The provider row had a `<button>` that was only as wide as its text content. Clicks on the empty area of the row were ignored, making it appear that providers could not be selected. Added `flex-1` to the button so it fills the available width inside the list item.

