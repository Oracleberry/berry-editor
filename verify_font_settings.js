// Font Settings Verification Script
// Run this in browser console to verify settings

console.log('🎨 Font Settings Verification');
console.log('================================');

// Check Canvas element
const canvas = document.querySelector('.berry-editor-pane canvas');
if (canvas) {
    console.log('✅ Canvas element found');

    // Get computed styles
    const computedStyle = window.getComputedStyle(canvas);
    console.log('Canvas computed styles:');
    console.log('  - image-rendering:', computedStyle.imageRendering);
    console.log('  - text-rendering:', computedStyle.textRendering);
    console.log('  - -webkit-font-smoothing:', computedStyle.webkitFontSmoothing);

    // Get canvas context and check font
    const ctx = canvas.getContext('2d');
    if (ctx) {
        console.log('✅ Canvas 2D context obtained');
        console.log('  - Current font:', ctx.font);

        // Verify expected font settings
        const expectedFont = '300 13px \'JetBrains Mono\'';
        const fontMatch = ctx.font.includes('13px') && ctx.font.includes('JetBrains Mono');

        if (fontMatch) {
            console.log('✅ Font settings are correct!');
            console.log('   Expected: 300 13px JetBrains Mono');
            console.log('   Actual:', ctx.font);
        } else {
            console.warn('⚠️ Font settings mismatch');
            console.log('   Expected: 300 13px JetBrains Mono');
            console.log('   Actual:', ctx.font);
        }

        // Check DPR
        const dpr = window.devicePixelRatio;
        console.log('  - Device Pixel Ratio:', dpr);
        console.log('  - Canvas width (CSS):', canvas.style.width);
        console.log('  - Canvas height (CSS):', canvas.style.height);
        console.log('  - Canvas width (physical):', canvas.width, 'px');
        console.log('  - Canvas height (physical):', canvas.height, 'px');

        const expectedPhysicalWidth = parseInt(canvas.style.width) * dpr;
        const widthMatch = Math.abs(canvas.width - expectedPhysicalWidth) < 2;

        if (widthMatch) {
            console.log('✅ DPR scaling is correct!');
        } else {
            console.warn('⚠️ DPR scaling mismatch');
            console.log('   Expected physical width:', expectedPhysicalWidth);
            console.log('   Actual physical width:', canvas.width);
        }
    }
} else {
    console.error('❌ Canvas element not found');
}

// Check body styles
const bodyStyle = window.getComputedStyle(document.body);
console.log('\nBody computed styles:');
console.log('  - font-family:', bodyStyle.fontFamily);
console.log('  - -webkit-font-smoothing:', bodyStyle.webkitFontSmoothing);
console.log('  - text-rendering:', bodyStyle.textRendering);
console.log('  - letter-spacing:', bodyStyle.letterSpacing);

// Check keyword styles
const keywords = document.querySelectorAll('.syntax-keyword');
if (keywords.length > 0) {
    console.log('\n✅ Found', keywords.length, 'keyword elements');
    const keywordStyle = window.getComputedStyle(keywords[0]);
    console.log('Keyword styles:');
    console.log('  - font-weight:', keywordStyle.fontWeight);
    console.log('  - color:', keywordStyle.color);

    if (keywordStyle.fontWeight === '500') {
        console.log('✅ Keyword font-weight is correct (500)');
    } else {
        console.warn('⚠️ Keyword font-weight mismatch');
        console.log('   Expected: 500');
        console.log('   Actual:', keywordStyle.fontWeight);
    }
}

console.log('\n================================');
console.log('Verification complete!');
