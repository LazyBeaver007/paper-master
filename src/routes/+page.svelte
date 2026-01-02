<!-- src/routes/+page.svelte -->
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount, onDestroy } from 'svelte';
  import * as pdfjsLib from 'pdfjs-dist';
  import pdfjsWorker from 'pdfjs-dist/build/pdf.worker?url';
  import 'pdfjs-dist/web/pdf_viewer.css'; // ← OFFICIAL PDF.js CSS

  pdfjsLib.GlobalWorkerOptions.workerSrc = pdfjsWorker;

  interface Paper {
    id: number;
    title: string;
    pdf_path: string;
    created_at: string | null;
  }

  interface Excerpt {
    id: number;
    text: string;
    page: number;
  }

  let papers: Paper[] = [];
  let selectedPaper: Paper | null = null;
  let message = '';
  let loading = false;

  // PDF.js state
  let pdfDoc: any = null;
  let pageNum = 1;
  let numPages = 0;
  let scale = 1.3;
  let canvas: HTMLCanvasElement;
  let textLayerDiv: HTMLDivElement;
  let ctx: CanvasRenderingContext2D;
  let pdfLoading = false;

  // Excerpts
  let excerpts: Excerpt[] = [];
  let nextExcerptId = 1;

  // Selection UI
  let showFloatingButton = false;
  let floatingButtonX = 0;
  let floatingButtonY = 0;
  let selectedText = '';

  async function loadPapers() {
    try {
      papers = await invoke<Paper[]>('get_papers');
      message = papers.length === 0 ? 'No papers yet — add your first one!' : '';
    } catch (err) {
      message = `Error loading library: ${err}`;
    }
  }

  async function addPaper() {
    loading = true;
    message = 'Select a PDF to add...';
    try {
      const result = await invoke<string>('add_paper');
      message = result;
      await loadPapers();
    } catch (err) {
      message = `Error: ${err}`;
    } finally {
      loading = false;
    }
  }

  async function openPaper(paper: Paper) {
    selectedPaper = paper;
    excerpts = [];
    nextExcerptId = 1;
    message = `Loading "${paper.title}"...`;

    try {
      const bytes: Uint8Array = await invoke('read_pdf_file', { path: paper.pdf_path });
      const loadingTask = pdfjsLib.getDocument({ data: bytes });
      pdfDoc = await loadingTask.promise;

      numPages = pdfDoc.numPages;
      pageNum = 1;
      message = '';
      renderPage(pageNum);
    } catch (err) {
      message = `Failed to load PDF: ${err}`;
      console.error(err);
    }
  }

  async function renderPage(num: number) {
    pdfLoading = true;
    const page = await pdfDoc.getPage(num);

    const viewport = page.getViewport({ scale });
    const outputScale = window.devicePixelRatio || 1;

    // Canvas
    canvas.width = Math.floor(viewport.width * outputScale);
    canvas.height = Math.floor(viewport.height * outputScale);
    canvas.style.width = viewport.width + 'px';
    canvas.style.height = viewport.height + 'px';

    ctx = canvas.getContext('2d')!;
    ctx.scale(outputScale, outputScale);

    const renderContext = {
      canvasContext: ctx,
      viewport: viewport
    };
    await page.render(renderContext).promise;

    // Text layer (OFFICIAL PDF.js)
    textLayerDiv.innerHTML = '';
    const textContent = await page.getTextContent();
    pdfjsLib.renderTextLayer({
      textContentSource: textContent,
      container: textLayerDiv,
      viewport: viewport,
      textDivs: []
    }).promise.then(() => {
      setupTextSelection();
    });

    pdfLoading = false;
  }

  function setupTextSelection() {
    textLayerDiv.style.userSelect = 'text';
    textLayerDiv.addEventListener('mouseup', handleTextSelection);
  }

  function handleTextSelection() {
    const selection = window.getSelection();
    if (!selection || selection.isCollapsed) {
      showFloatingButton = false;
      return;
    }

    selectedText = selection.toString().trim();
    if (selectedText.length < 3) {
      showFloatingButton = false;
      return;
    }

    const range = selection.getRangeAt(0);
    const rect = range.getBoundingClientRect();
    const containerRect = textLayerDiv.getBoundingClientRect();

    floatingButtonX = rect.left - containerRect.left + rect.width / 2;
    floatingButtonY = rect.top - containerRect.top - 40;

    showFloatingButton = true;
  }

  function createExcerpt() {
    if (!selectedText) return;

    excerpts = [...excerpts, {
      id: nextExcerptId++,
      text: selectedText,
      page: pageNum
    }];

    window.getSelection()?.removeAllRanges();
    showFloatingButton = false;
    selectedText = '';
  }

  function nextPage() {
    if (pageNum >= numPages) return;
    pageNum++;
    renderPage(pageNum);
  }

  function prevPage() {
    if (pageNum <= 1) return;
    pageNum--;
    renderPage(pageNum);
  }

  function zoomIn() {
    scale += 0.2;
    renderPage(pageNum);
  }

  function zoomOut() {
    scale -= 0.2;
    if (scale < 0.5) scale = 0.5;
    renderPage(pageNum);
  }

  function backToLibrary() {
    selectedPaper = null;
    pdfDoc = null;
    excerpts = [];
    showFloatingButton = false;
  }

  onMount(() => {
    loadPapers();
    document.addEventListener('click', (e) => {
      if (showFloatingButton && !e.target.closest('.floating-excerpt-btn')) {
        showFloatingButton = false;
      }
    });
  });

  onDestroy(() => {
    if (pdfDoc) pdfDoc.destroy();
  });
</script>

<main class="app">
  {#if !selectedPaper}
    <!-- Library View -->
    <div class="library">
      <header class="header">
        <h1>PaperMaster</h1>
        <p class="tagline">Master research papers with incremental reading & spaced repetition</p>
      </header>

      <div class="toolbar">
        <button on:click={addPaper} class="btn-primary" disabled={loading}>
          {#if loading} Adding... {:else} + Add New Paper {/if}
        </button>
        <button on:click={loadPapers} class="btn-secondary">Refresh</button>
      </div>

      {#if message && papers.length === 0}
        <div class="empty-state">
          <p>{message}</p>
        </div>
      {/if}

      {#if papers.length > 0}
        <div class="grid">
          {#each papers as paper (paper.id)}
            <div class="card" role="button" tabindex="0" on:click={() => openPaper(paper)}>
              <div class="card-content">
                <div class="title">{paper.title}</div>
                <div class="meta">
                  Added {new Date(paper.created_at || '').toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' })}
                </div>
              </div>
              <button class="open-btn" on:click|stopPropagation={() => openPaper(paper)}>
                Open →
              </button>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {:else}
    <!-- Reader Split View -->
    <div class="reader">
      <header class="reader-header">
        <button on:click={backToLibrary} class="back-btn">← Library</button>
        <h2>{selectedPaper.title}</h2>
        <div class="spacer"></div>
      </header>

      <div class="split-view">
        <div class="pdf-pane">
          <div class="pdf-controls">
            <button on:click={zoomOut} disabled={scale <= 0.5}>−</button>
            <span>{Math.round(scale * 100)}%</span>
            <button on:click={zoomIn} disabled={scale >= 3}>+</button>
            <span class="page-info">Page {pageNum} of {numPages}</span>
            <button on:click={prevPage} disabled={pageNum <= 1}>‹</button>
            <button on:click={nextPage} disabled={pageNum >= numPages}>›</button>
          </div>

          <div class="pdf-viewer-container">
            <div class="pdf-canvas-container">
              <canvas bind:this={canvas}></canvas>
              <div class="textLayer" bind:this={textLayerDiv}></div>
            </div>

            {#if pdfLoading}
              <div class="pdf-overlay">Loading page {pageNum}...</div>
            {/if}

            {#if showFloatingButton}
              <button
                class="floating-excerpt-btn"
                style="left: {floatingButtonX}px; top: {floatingButtonY}px;"
                on:click|stopPropagation={createExcerpt}
              >
                + Create Excerpt
              </button>
            {/if}
          </div>
        </div>

        <div class="notes-pane">
          <div class="notes-header">
            <h3>Excerpts & Notes ({excerpts.length})</h3>
          </div>

          {#if excerpts.length === 0}
            <div class="notes-placeholder">
              <p>Select text in the PDF → click "+ Create Excerpt"</p>
              <p class="hint">These will become your flashcards for spaced repetition</p>
            </div>
          {:else}
            <div class="excerpts-list">
              {#each excerpts as excerpt (excerpt.id)}
                <div class="excerpt-item">
                  <p class="excerpt-text">"{excerpt.text}"</p>
                  <p class="excerpt-page">— Page {excerpt.page}</p>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      </div>
    </div>
  {/if}

  {#if message && loading}
    <div class="toast">{message}</div>
  {/if}
</main>

<style>
  :global(body) {
    margin: 0;
    background: #0f0f17;
    color: #e0e0e0;
    font-family: 'Inter', system-ui, -apple-system, sans-serif;
  }

  .app {
    min-height: 100vh;
  }

  /* Library View */
  .library {
    padding: 2rem;
    max-width: 1200px;
    margin: 0 auto;
  }

  .header {
    text-align: center;
    margin-bottom: 3rem;
  }

  h1 {
    font-size: 3.5rem;
    font-weight: 800;
    background: linear-gradient(90deg, #ff6b6b, #da1b60);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    margin: 0;
  }

  .tagline {
    font-size: 1.2rem;
    color: #888;
    margin-top: 0.5rem;
  }

  .toolbar {
    display: flex;
    justify-content: center;
    gap: 1rem;
    margin-bottom: 2rem;
  }

  .btn-primary, .btn-secondary, .btn-small {
    padding: 0.8rem 1.6rem;
    border: none;
    border-radius: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-primary {
    background: #da1b60;
    color: white;
    font-size: 1.1rem;
  }

  .btn-primary:hover:not(:disabled) {
    background: #c01852;
    transform: translateY(-2px);
  }

  .btn-secondary {
    background: #252533;
    color: #ccc;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: 1.5rem;
  }

  .card {
    background: #1a1a24;
    border-radius: 16px;
    overflow: hidden;
    box-shadow: 0 4px 20px rgba(0,0,0,0.3);
    transition: all 0.3s;
    cursor: pointer;
    display: flex;
    flex-direction: column;
  }

  .card:hover {
    transform: translateY(-8px);
    box-shadow: 0 12px 30px rgba(218, 27, 96, 0.2);
  }

  .card-content {
    padding: 1.5rem;
    flex-grow: 1;
  }

  .title {
    font-size: 1.3rem;
    font-weight: 600;
    margin-bottom: 0.5rem;
    color: white;
  }

  .meta {
    font-size: 0.95rem;
    color: #999;
  }

  .open-btn {
    background: #da1b60;
    color: white;
    border: none;
    padding: 1rem;
    font-weight: 600;
    cursor: pointer;
  }

  .open-btn:hover {
    background: #c01852;
  }

  .empty-state {
    text-align: center;
    padding: 4rem;
    color: #666;
    font-size: 1.2rem;
  }

  /* Reader View */
  .reader {
    height: 100vh;
    display: flex;
    flex-direction: column;
  }

  .reader-header {
    padding: 1rem 2rem;
    background: #14141c;
    border-bottom: 1px solid #252533;
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .back-btn {
    background: none;
    border: none;
    color: #aaa;
    font-size: 1.5rem;
    cursor: pointer;
  }

  .reader-header h2 {
    margin: 0;
    font-size: 1.4rem;
    color: white;
    flex-grow: 1;
  }

  .spacer { flex-grow: 1; }

  .split-view {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .pdf-pane {
    flex: 6;
    background: #000;
    display: flex;
    flex-direction: column;
  }

  .pdf-controls {
    padding: 1rem;
    background: #14141c;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 1rem;
    flex-wrap: wrap;
    font-size: 0.9rem;
  }

  .pdf-controls button {
    background: #252533;
    color: white;
    border: none;
    width: 36px;
    height: 36px;
    border-radius: 8px;
    cursor: pointer;
    font-size: 1.2rem;
  }

  .pdf-controls button:hover:not(:disabled) {
    background: #da1b60;
  }

  .pdf-controls button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .page-info {
    color: #aaa;
    min-width: 120px;
    text-align: center;
  }

  .pdf-viewer-container {
    position: relative;
    flex: 1;
    overflow: hidden;
  }

  .pdf-canvas-container {
    position: relative;
    flex: 1;
    overflow: auto;
    background: #000;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding: 1rem;
    height: 100%;
  }

  /* PERFECT TEXT LAYER - OFFICIAL PDF.js + our selection color */
  .textLayer {
    position: absolute;
    inset: 0;
    overflow: hidden;
    opacity: 0.2;
    line-height: 1;
    max-width: 100%;
    max-height: 100%;
  }

  .textLayer > span {
    color: transparent;
    position: absolute;
    white-space: pre;
    cursor: text;
    transform-origin: 0% 0%;
  }

  .textLayer ::selection {
    background: rgba(218, 27, 96, 0.4);
  }

  .pdf-overlay {
    position: absolute;
    inset: 0;
    background: rgba(0,0,0,0.7);
    color: white;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 1.2rem;
    z-index: 10;
  }

  .floating-excerpt-btn {
    position: absolute;
    background: #da1b60;
    color: white;
    border: none;
    padding: 0.6rem 1.2rem;
    border-radius: 8px;
    font-weight: 600;
    font-size: 0.9rem;
    cursor: pointer;
    transform: translateX(-50%);
    box-shadow: 0 8px 25px rgba(218, 27, 96, 0.4);
    z-index: 100;
    white-space: nowrap;
  }

  .floating-excerpt-btn:hover {
    background: #c01852;
  }

  .notes-pane {
    flex: 4;
    background: #1e1e28;
    border-left: 1px solid #252533;
    display: flex;
    flex-direction: column;
  }

  .notes-header {
    padding: 1.5rem;
    border-bottom: 1px solid #252533;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .notes-header h3 {
    margin: 0;
    font-size: 1.3rem;
  }

  .btn-small {
    background: #2a2a38;
    color: #ccc;
    font-size: 0.9rem;
    padding: 0.6rem 1rem;
    border-radius: 8px;
  }

  .notes-placeholder {
    flex-grow: 1;
    padding: 3rem 2rem;
    color: #888;
    text-align: center;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 1rem;
  }

  .hint {
    font-size: 0.95rem;
    color: #666;
  }

  .excerpts-list {
    padding: 1.5rem;
    overflow-y: auto;
    flex: 1;
  }

  .excerpt-item {
    background: #252533;
    padding: 1.2rem;
    border-radius: 12px;
    margin-bottom: 1rem;
    border-left: 4px solid #da1b60;
    transition: all 0.2s;
  }

  .excerpt-item:hover {
    background: #2a2a38;
  }

  .excerpt-text {
    margin: 0 0 0.5rem 0;
    font-style: italic;
    color: #ddd;
    line-height: 1.4;
  }

  .excerpt-page {
    margin: 0;
    font-size: 0.9rem;
    color: #aaa;
    font-weight: 500;
  }

  .toast {
    position: fixed;
    bottom: 2rem;
    left: 50%;
    transform: translateX(-50%);
    background: #da1b60;
    color: white;
    padding: 1rem 2rem;
    border-radius: 12px;
    box-shadow: 0 8px 20px rgba(0,0,0,0.4);
  }
</style>
