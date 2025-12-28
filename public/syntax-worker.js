/**
 * Syntax Analysis Web Worker
 *
 * Runs in a separate thread to achieve 144fps UI guarantee
 * IntelliJ-beating strategy: ZERO impact on main thread
 */

// Worker state
let parserState = {
    isAnalyzing: false,
    currentLanguage: 'rust',
    analysisQueue: [],
    cache: new Map(), // line_number -> highlighted_html
};

/**
 * Message handler
 * Expected format:
 * {
 *   type: 'highlight_lines' | 'clear_cache' | 'set_language',
 *   data: { ... }
 * }
 */
self.onmessage = async function(e) {
    const { type, data } = e.data;

    try {
        switch (type) {
            case 'highlight_lines':
                await highlightLines(data.lines, data.language || parserState.currentLanguage);
                break;

            case 'highlight_single_line':
                await highlightSingleLine(data.lineNumber, data.text, data.language || parserState.currentLanguage);
                break;

            case 'clear_cache':
                parserState.cache.clear();
                postMessage({ type: 'cache_cleared' });
                break;

            case 'set_language':
                parserState.currentLanguage = data.language;
                postMessage({ type: 'language_set', language: data.language });
                break;

            case 'get_cache_stats':
                postMessage({
                    type: 'cache_stats',
                    size: parserState.cache.size,
                    isAnalyzing: parserState.isAnalyzing
                });
                break;

            default:
                postError(`Unknown message type: ${type}`);
        }
    } catch (error) {
        postError(`Worker error: ${error.message}`);
    }
};

/**
 * Highlight multiple lines (batch processing)
 */
async function highlightLines(lines, language) {
    parserState.isAnalyzing = true;
    const results = [];

    for (const { lineNumber, text } of lines) {
        // Check cache first
        const cacheKey = `${language}:${lineNumber}:${text}`;
        if (parserState.cache.has(cacheKey)) {
            results.push({
                lineNumber,
                html: parserState.cache.get(cacheKey)
            });
            continue;
        }

        // Perform syntax highlighting
        const html = highlightRustLine(text);

        // Cache result
        parserState.cache.set(cacheKey, html);

        results.push({ lineNumber, html });

        // Yield to event loop every 10 lines to prevent blocking
        if (results.length % 10 === 0) {
            await new Promise(resolve => setTimeout(resolve, 0));
        }
    }

    parserState.isAnalyzing = false;

    postMessage({
        type: 'highlight_result',
        results
    });
}

/**
 * Highlight single line (for real-time typing)
 */
async function highlightSingleLine(lineNumber, text, language) {
    const html = highlightRustLine(text);

    postMessage({
        type: 'single_line_result',
        lineNumber,
        html
    });
}

/**
 * IntelliJ Darcula color scheme syntax highlighting
 * Optimized regex-based parser (will be replaced with tree-sitter in Task 2)
 */
function highlightRustLine(line) {
    const keywords = [
        'fn', 'let', 'mut', 'const', 'static', 'impl', 'trait', 'struct', 'enum',
        'mod', 'pub', 'use', 'crate', 'self', 'super', 'async', 'await', 'move',
        'if', 'else', 'match', 'loop', 'while', 'for', 'in', 'return', 'break',
        'continue', 'as', 'ref', 'where', 'unsafe', 'extern', 'type', 'dyn',
    ];

    const types = [
        'String', 'str', 'usize', 'isize', 'f64', 'f32', 'i32', 'u32',
        'i64', 'u64', 'bool', 'Vec', 'Option', 'Result', 'Some', 'None',
        'Ok', 'Err', 'Box', 'Rc', 'Arc', 'RefCell', 'RwSignal'
    ];

    let result = '';
    let i = 0;

    // Fast path: check for comments first
    const commentIdx = line.indexOf('//');
    if (commentIdx !== -1) {
        if (commentIdx > 0) {
            result += highlightRustLineInternal(line.substring(0, commentIdx), keywords, types);
        }
        result += `<span style="color: #6A9955">${escapeHtml(line.substring(commentIdx))}</span>`;
        return result;
    }

    return highlightRustLineInternal(line, keywords, types);
}

/**
 * Internal highlighting logic
 */
function highlightRustLineInternal(line, keywords, types) {
    let result = '';
    let i = 0;

    while (i < line.length) {
        const ch = line[i];

        // String literals
        if (ch === '"') {
            let str = '"';
            i++;
            while (i < line.length) {
                const c = line[i];
                str += c;
                i++;
                if (c === '"' && line[i - 2] !== '\\') break;
            }
            result += `<span style="color: #CE9178">${escapeHtml(str)}</span>`;
            continue;
        }

        // Numbers
        if (ch >= '0' && ch <= '9') {
            let num = '';
            while (i < line.length && (line[i].match(/[0-9.]/) || line[i] === '_')) {
                num += line[i];
                i++;
            }
            result += `<span style="color: #B5CEA8">${escapeHtml(num)}</span>`;
            continue;
        }

        // Identifiers and keywords
        if ((ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z') || ch === '_') {
            let word = '';
            while (i < line.length) {
                const c = line[i];
                if ((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') ||
                    (c >= '0' && c <= '9') || c === '_') {
                    word += c;
                    i++;
                } else {
                    break;
                }
            }

            let color = '#A9B7C6'; // Default
            if (keywords.includes(word)) {
                color = '#CC7832'; // Keyword
            } else if (types.includes(word)) {
                color = '#4EC9B0'; // Type
            } else if (word[0] >= 'A' && word[0] <= 'Z') {
                color = '#4EC9B0'; // Type (capitalized)
            }

            result += `<span style="color: ${color}">${escapeHtml(word)}</span>`;
            continue;
        }

        // Other characters
        result += escapeHtml(ch);
        i++;
    }

    return result;
}

/**
 * HTML escape
 */
function escapeHtml(text) {
    const map = {
        '&': '&amp;',
        '<': '&lt;',
        '>': '&gt;',
        '"': '&quot;',
        "'": '&#39;'
    };
    return text.replace(/[&<>"']/g, m => map[m]);
}

/**
 * Send error to main thread
 */
function postError(message) {
    self.postMessage({
        type: 'error',
        error: message
    });
}

// Signal worker is ready
self.postMessage({ type: 'ready' });
