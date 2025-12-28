/**
 * Web Worker for Background Symbol Indexing
 *
 * Runs in a separate thread to avoid blocking the UI
 * Communicates with main thread via postMessage
 */

// Worker state
let indexingState = {
    isIndexing: false,
    totalSymbols: 0,
    processedFiles: 0,
    errors: []
};

/**
 * Message handler
 * Expected message format:
 * {
 *   type: 'index_workspace' | 'search_symbols' | 'get_status',
 *   data: { ... }
 * }
 */
self.onmessage = async function(e) {
    const { type, data } = e.data;

    try {
        switch (type) {
            case 'index_workspace':
                await indexWorkspace(data.path, data.apiEndpoint);
                break;

            case 'search_symbols':
                await searchSymbols(data.query, data.apiEndpoint);
                break;

            case 'get_status':
                postStatus();
                break;

            case 'cancel':
                cancelIndexing();
                break;

            default:
                postError(`Unknown message type: ${type}`);
        }
    } catch (error) {
        postError(`Worker error: ${error.message}`);
    }
};

/**
 * Index workspace by calling Tauri backend
 */
async function indexWorkspace(workspacePath, apiEndpoint) {
    if (indexingState.isIndexing) {
        postError('Indexing already in progress');
        return;
    }

    indexingState.isIndexing = true;
    indexingState.processedFiles = 0;
    indexingState.totalSymbols = 0;
    indexingState.errors = [];

    postProgress({ status: 'started', message: 'Starting workspace indexing...' });

    try {
        // Call Tauri backend via fetch (Web Worker can't use window.__TAURI__)
        // Instead, we'll use Tauri's HTTP API if available, or signal main thread

        // For now, signal main thread to call Tauri
        postMessage({
            type: 'call_tauri',
            command: 'index_workspace',
            args: { path: workspacePath }
        });

        // Note: The main thread will receive the Tauri response and forward it back
        // This is a limitation of Web Workers - they can't directly access window objects

    } catch (error) {
        indexingState.isIndexing = false;
        postError(`Indexing failed: ${error.message}`);
    }
}

/**
 * Handle indexing result from main thread
 */
self.handleIndexingResult = function(result) {
    indexingState.isIndexing = false;

    if (result.success) {
        indexingState.totalSymbols = result.symbolCount;
        postProgress({
            status: 'completed',
            message: `Indexed ${result.symbolCount} symbols`,
            symbolCount: result.symbolCount
        });
    } else {
        postError(`Indexing failed: ${result.error}`);
    }
};

/**
 * Search symbols
 */
async function searchSymbols(query, apiEndpoint) {
    postMessage({
        type: 'call_tauri',
        command: 'search_symbols',
        args: { query }
    });
}

/**
 * Handle search result from main thread
 */
self.handleSearchResult = function(result) {
    if (result.success) {
        postMessage({
            type: 'search_result',
            symbols: result.symbols
        });
    } else {
        postError(`Search failed: ${result.error}`);
    }
};

/**
 * Cancel ongoing indexing
 */
function cancelIndexing() {
    indexingState.isIndexing = false;
    postProgress({ status: 'cancelled', message: 'Indexing cancelled' });
}

/**
 * Send progress update to main thread
 */
function postProgress(data) {
    self.postMessage({
        type: 'progress',
        data: {
            ...data,
            isIndexing: indexingState.isIndexing,
            totalSymbols: indexingState.totalSymbols,
            processedFiles: indexingState.processedFiles
        }
    });
}

/**
 * Send status to main thread
 */
function postStatus() {
    self.postMessage({
        type: 'status',
        data: {
            isIndexing: indexingState.isIndexing,
            totalSymbols: indexingState.totalSymbols,
            processedFiles: indexingState.processedFiles,
            errors: indexingState.errors
        }
    });
}

/**
 * Send error to main thread
 */
function postError(message) {
    indexingState.errors.push({ message, timestamp: Date.now() });
    self.postMessage({
        type: 'error',
        error: message
    });
}

// Signal worker is ready
self.postMessage({ type: 'ready' });
