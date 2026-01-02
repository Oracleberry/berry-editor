// Terminal Focus Debug Script
// Open http://localhost:8080 in browser, then run this in console:

console.log("🔍 Starting terminal focus debug...");

// Wait a bit for the app to load
setTimeout(() => {
    console.log("\n📊 Checking terminal panel state:");

    // Check if terminal panel exists
    const panel = document.querySelector('.terminal-panel');
    console.log("Terminal panel exists:", !!panel);

    // Check if input exists
    const input = document.querySelector('.terminal-input input[type="text"]');
    console.log("Input element exists:", !!input);

    if (input) {
        console.log("\n📋 Input element details:");
        console.log("- Type:", input.type);
        console.log("- Placeholder:", input.placeholder);
        console.log("- Disabled:", input.disabled);
        console.log("- ReadOnly:", input.readOnly);
        console.log("- tabIndex:", input.tabIndex);

        // Check computed style
        const style = window.getComputedStyle(input);
        console.log("- display:", style.display);
        console.log("- visibility:", style.visibility);
        console.log("- pointer-events:", style.pointerEvents);
        console.log("- opacity:", style.opacity);
        console.log("- z-index:", style.zIndex);

        // Check focus state
        const isFocused = document.activeElement === input;
        console.log("\n🎯 Focus state:");
        console.log("- Is focused:", isFocused);
        console.log("- Active element:", document.activeElement?.tagName, document.activeElement?.type);

        // Try to focus
        console.log("\n🔧 Attempting to focus...");
        input.focus();

        setTimeout(() => {
            const nowFocused = document.activeElement === input;
            console.log("✅ Focus after focus():", nowFocused);

            if (!nowFocused) {
                console.error("❌ Focus failed! Active element is:", document.activeElement);

                // Check for overlapping elements
                const rect = input.getBoundingClientRect();
                const elemAtPoint = document.elementFromPoint(
                    rect.left + rect.width / 2,
                    rect.top + rect.height / 2
                );
                console.log("🔍 Element at input position:", elemAtPoint?.tagName, elemAtPoint?.className);

                if (elemAtPoint !== input) {
                    console.error("⚠️ Input is covered by another element!");
                }
            } else {
                console.log("✅ Input is now focused! Try typing...");
            }
        }, 100);
    } else {
        console.error("❌ Input element not found!");
        console.log("Available elements:", document.querySelectorAll('.terminal-input *'));
    }
}, 1000);
