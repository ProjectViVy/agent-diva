/**
 * Manages a draggable DOM overlay element for displaying text subtitles
 * with fade transitions.
 *
 * Pure DOM controller with no Three.js or VRM dependency.
 * Inspired by the subtitle implementation in `super-agent-party/vrm.js`.
 *
 * @example
 * ```ts
 * const controller = new SubtitleController(document.body);
 * controller.setText('Hello world');
 * controller.clear();
 * controller.destroy();
 * ```
 */
export class SubtitleController {
    /** Parent element the subtitle overlay was appended to. */
    private readonly container: HTMLElement;

    /** The subtitle overlay DOM element. */
    private readonly element: HTMLDivElement;

    /** Whether the subtitle is currently set to be visible. */
    private _visible = false;

    /** Whether a drag operation is in progress. */
    private isDragging = false;

    /** Horizontal offset from the element center to the mouse position when drag began. */
    private dragOffsetX = 0;

    /** Vertical offset from the element top to the mouse position when drag began. */
    private dragOffsetY = 0;

    /** Timeout ID used to delay text clearing after the fade-out transition. */
    private fadeTimeoutId: ReturnType<typeof setTimeout> | null = null;

    // Bound event handlers so we can cleanly add/remove listeners.
    private readonly _onMouseDown: (e: MouseEvent) => void;
    private readonly _onMouseMove: (e: MouseEvent) => void;
    private readonly _onMouseUp: () => void;

    /**
     * Creates a `SubtitleController` and appends the styled subtitle overlay
     * to the given container element.
     *
     * @param container - The parent `HTMLElement` to attach the subtitle overlay to.
     *                    Must be a valid DOM element (throws otherwise).
     */
    constructor(container: HTMLElement) {
        if (!container) {
            throw new Error(
                'SubtitleController: container must be a valid HTMLElement',
            );
        }
        this.container = container;

        // ---- Create the overlay element ----------------------------------
        this.element = document.createElement('div');
        this.element.id = 'subtitle-container';

        const s = this.element.style;
        s.position = 'fixed';
        s.top = '50%';
        s.left = '50%';
        s.transform = 'translateX(-50%)';
        s.width = 'max-content';
        s.maxWidth = '80%';
        s.minWidth = '100px';
        s.padding = '12px 24px';
        s.background = 'rgba(0, 0, 0, 0.8)';
        s.color = 'white';
        s.borderRadius = '8px';
        s.fontFamily = 'Arial, sans-serif';
        s.fontSize = '1.2em';
        s.textAlign = 'center';
        s.backdropFilter = 'blur(10px)';
        s.opacity = '0';
        s.transition = 'opacity 0.3s ease, transform 0.3s ease';
        s.zIndex = '9998';
        s.whiteSpace = 'pre-wrap';
        s.lineHeight = '1.5';
        s.cursor = 'move';
        s.userSelect = 'none';
        s.display = 'none';

        this.container.appendChild(this.element);

        // ---- Bind drag handlers ------------------------------------------
        this._onMouseDown = this._handleMouseDown.bind(this);
        this._onMouseMove = this._handleMouseMove.bind(this);
        this._onMouseUp = this._handleMouseUp.bind(this);

        this.element.addEventListener('mousedown', this._onMouseDown);
    }

    /**
     * Show or hide the subtitle element.
     *
     * @param visible - `true` to show, `false` to hide.
     */
    setVisible(visible: boolean): void {
        this._visible = visible;
        this.element.style.display = visible ? 'block' : 'none';
    }

    /**
     * Update the subtitle text content and trigger a fade-in transition.
     *
     * If the element is currently hidden, it is shown first. Any pending
     * fade-out timeout is cancelled so the new text appears immediately.
     *
     * @param text       - The subtitle text to display.
     * @param chunkIndex - Optional index for chunked subtitle delivery (reserved for future use).
     */
    setText(text: string, chunkIndex?: number): void {
        // Ensure the element is visible before showing text.
        if (!this._visible) {
            this.setVisible(true);
        }

        // Cancel any in-progress fade-out.
        if (this.fadeTimeoutId !== null) {
            clearTimeout(this.fadeTimeoutId);
            this.fadeTimeoutId = null;
        }

        // Avoid no-unused-vars warning for the reserved parameter.
        void chunkIndex;

        this.element.textContent = text;

        // Force a layout reflow so the browser picks up the opacity change
        // as a new transition even when the previous value was also '1'.
        // eslint-disable-next-line @typescript-eslint/no-unused-expressions
        void this.element.offsetHeight;

        this.element.style.opacity = '1';
    }

    /**
     * Fade out the subtitle text (opacity → 0) and clear its content
     * after the transition completes. The element itself stays in the DOM
     * so it can be reused on the next {@link setText} call.
     */
    clear(): void {
        this.element.style.opacity = '0';

        // After the CSS transition (300ms), clear the text and hide the element.
        if (this.fadeTimeoutId !== null) {
            clearTimeout(this.fadeTimeoutId);
        }
        this.fadeTimeoutId = setTimeout(() => {
            this.element.textContent = '';
            this.element.style.display = 'none';
            this._visible = false;
            this.fadeTimeoutId = null;
        }, 300);
    }

    /**
     * Returns the current visibility state as last set by {@link setVisible}
     * or as a side-effect of {@link setText}.
     */
    isVisible(): boolean {
        return this._visible;
    }

    /**
     * Remove the subtitle DOM element from its container and clean up
     * all event listeners and pending timeouts.
     *
     * Call this when the controller is no longer needed to prevent leaks.
     */
    destroy(): void {
        // Clear any pending fade-out timeout.
        if (this.fadeTimeoutId !== null) {
            clearTimeout(this.fadeTimeoutId);
            this.fadeTimeoutId = null;
        }

        // Remove drag listeners.
        this.element.removeEventListener('mousedown', this._onMouseDown);
        document.removeEventListener('mousemove', this._onMouseMove);
        document.removeEventListener('mouseup', this._onMouseUp);

        // Remove the element from the DOM.
        if (this.element.parentNode) {
            this.element.parentNode.removeChild(this.element);
        }
    }

    // ----------------------------------------------------------------
    //  Drag-to-reposition
    // ----------------------------------------------------------------

    /**
     * Records the mouse offset relative to the element center (X) and
     * top (Y), then begins tracking mouse movement for drag.
     */
    private _handleMouseDown(e: MouseEvent): void {
        e.preventDefault();
        this.isDragging = true;

        const rect = this.element.getBoundingClientRect();

        // Reference-style offset calculation:
        //   X: distance from element horizontal center to mouse
        //   Y: distance from element top to mouse
        this.dragOffsetX = e.clientX - (rect.left + rect.width / 2);
        this.dragOffsetY = e.clientY - rect.top;

        // Disable CSS transition during drag for instant, snappy movement.
        this.element.style.transition = 'none';

        document.addEventListener('mousemove', this._onMouseMove);
        document.addEventListener('mouseup', this._onMouseUp);
    }

    /**
     * Updates the element position based on the current mouse location,
     * applying the recorded offset and clamping within viewport bounds.
     */
    private _handleMouseMove(e: MouseEvent): void {
        if (!this.isDragging) return;

        const halfWidth = this.element.offsetWidth / 2;
        const height = this.element.offsetHeight;

        const centerX = e.clientX - this.dragOffsetX;
        const topY = e.clientY - this.dragOffsetY;

        // Clamp so the element stays fully inside the viewport.
        const clampedX = Math.max(
            halfWidth,
            Math.min(centerX, window.innerWidth - halfWidth),
        );
        const clampedY = Math.max(
            0,
            Math.min(topY, window.innerHeight - height),
        );

        // Use pixel-based positioning while dragging.
        // translateX(-50%) centers the element at its left edge,
        // matching the initial centred state.
        this.element.style.left = `${clampedX}px`;
        this.element.style.top = `${clampedY}px`;
        this.element.style.transform = 'translateX(-50%)';
    }

    /**
     * Ends the drag operation and restores the CSS transition property.
     */
    private _handleMouseUp(): void {
        if (!this.isDragging) return;

        this.isDragging = false;
        this.element.style.transition =
            'opacity 0.3s ease, transform 0.3s ease';

        document.removeEventListener('mousemove', this._onMouseMove);
        document.removeEventListener('mouseup', this._onMouseUp);
    }
}

export default SubtitleController
