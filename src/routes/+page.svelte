<!-- src/routes/+page.svelte - COMPLETE FILE -->
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount, onDestroy, tick } from 'svelte';
  import * as pdfjsLib from 'pdfjs-dist';
  import pdfjsWorker from 'pdfjs-dist/build/pdf.worker?url';

  // Setup PDF.js worker
  pdfjsLib.GlobalWorkerOptions.workerSrc = pdfjsWorker;

  interface Paper {
    id: number;
    title: string;
    pdf_path: string;
    created_at: string | null;
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
  let ctx: CanvasRenderingContext2D;
  let containerWidth = 0;
  let pdfLoading = false;

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
    message = `Loading "${paper.title}"...`;
    pdfLoading = true;
    pageNum = 1;

    try {
      const bytes: Uint8Array = await invoke('read_pdf_file', { path: paper.pdf_path });
      const loadingTask = pdfjsLib.getDocument({ data: bytes });
      pdfDoc = await loadingTask.promise;

      numPages = pdfDoc.numPages;
      await tick();
      renderPage(pageNum);
      message = '';
    } catch (err) {
      message = `Failed to load PDF: ${err}`;
      console.error('PDF load error:', err);
      pdfLoading = false;
    }
  }

  async function renderPage(num: number) {
    if (!pdfDoc || !canvas) return;
    
    pdfLoading = true;
    try {
      const page = await pdfDoc.getPage(num);
      const viewport = page.getViewport({ scale });
      const outputScale = window.devicePixelRatio || 1;

      canvas.width = Math.floor(viewport.width * outputScale);
      canvas.height = Math.floor(viewport.height * outputScale);
      canvas.style.width = `${viewport.width}px`;
      canvas.style.height = `${viewport.height}px`;

      ctx = canvas.getContext('2d')!;
      ctx.save();
      ctx.scale(outputScale, outputScale);

      const renderContext = {
        canvasContext: ctx,
        viewport: viewport
      };

      await page.render(renderContext).promise;
      ctx.restore();
      pageNum = num;
      pdfLoading = false;
    } catch (err) {
      console.error('Render error:', err);
      pdfLoading = false;
    }
  }

  function nextPage() {
    if (pageNum >= numPages || pdfLoading) return;
    pageNum++;
    renderPage(pageNum);
  }

  function prevPage() {
    if (pageNum <= 1 || pdfLoading) return;
    pageNum--;
    renderPage(pageNum);
  }

  function zoomIn() {
    if (scale >= 3 || pdfLoading) return;
    scale += 0.2;
    renderPage(pageNum);
  }

  function zoomOut() {
    if (scale <= 0.5 || pdfLoading) return;
    scale = Math.max(0.5, scale - 0.2);
    renderPage(pageNum);
  }

  function backToLibrary() {
    if (pdfDoc) {
      pdfDoc.destroy();
      pdfDoc = null;
    }
    selectedPaper = null;
    pageNum = 1;
    numPages = 0;
    scale = 1.3;
    pdfLoading = false;
    message = '';
  }

  // Keyboard shortcuts
  function handleKeydown(event: KeyboardEvent) {
    if (!selectedPaper) return;
    
    switch (event.code) {
      case 'ArrowLeft':
      case 'ArrowUp':
        event.preventDefault();
        prevPage();
        break;
      case 'ArrowRight':
      case 'ArrowDown':
        event.preventDefault();
        nextPage();
        break;
      case 'Equal':
      case 'NumpadAdd':
        event.preventDefault();
        zoomIn();
        break;
      case 'Minus':
      case 'NumpadSubtract':
        event.preventDefault();
        zoomOut();
        break;
    }
  }

  onMount(() => {
    loadPapers();
    window.addEventListener('keydown', handleKeydown);
  });

  onDestroy(() => {
    window.removeEventListener('keydown', handleKeydown);
    if (pdfDoc) {
      pdfDoc.destroy();
    }
  });
</script>

<svelte:window bind:innerWidth={containerWidth} />

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
          {#if loading}
            <span class="spinner"></span>
            Adding...
          {:else}
            + Add New Paper
          {/if}
        </button>
        <button on:click={loadPapers} class="btn-secondary" disabled={loading}>
          Refresh
        </button>
      </div>

      {#if papers.length === 0 && message}
        <div class="empty-state">
          <div class="empty-icon">📚</div>
          <p>{message}</p>
          <p class="empty-subtitle">Import your first research paper to get started</p>
        </div>
      {/if}

      {#if papers.length > 0}
        <div class="library-stats">
          <span class="stat">{papers.length} papers</span>
          <span class="stat">Ready to excerpt</span>
        </div>
        
        <div class="grid">
          {#each papers as paper (paper.id)}
            <div 
              class="card" 
              on:click={() => openPaper(paper)} 
              on:keydown={(e) => e.key === 'Enter' && openPaper(paper)}
              role="button" 
              tabindex="0"
            >
              <div class="card-content">
                <div class="title">{paper.title}</div>
                <div class="meta">
                  Added {new Date(paper.created_at || Date.now()).toLocaleDateString('en-US', { 
                    month: 'short', 
                    day: 'numeric', 
                    year: 'numeric' 
                  })}
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
        <button on:click={backToLibrary} class="back-btn" title="Back to Library (Esc)">
          ← Library
        </button>
        <div class="paper-info">
          <h2>{selectedPaper.title}</h2>
          <span class="page-counter">
            Page {pageNum} of {numPages}
          </span>
        </div>
        <div class="pdf-controls">
          <button on:click={zoomOut} disabled={scale <= 0.5 || pdfLoading} title="Zoom Out (-)">
            −
          </button>
          <span class="zoom-level">{Math.round(scale * 100)}%</span>
          <button on:click={zoomIn} disabled={scale >= 3 || pdfLoading} title="Zoom In (+)">
            +
          </button>
          <button on:click={prevPage} disabled={pageNum <= 1 || pdfLoading} title="Previous Page (←)">
            ‹
          </button>
          <button on:click={nextPage} disabled={pageNum >= numPages || pdfLoading} title="Next Page (→)">
            ›
          </button>
        </div>
      </header>

      <div class="split-view">
        <!-- PDF Viewer Pane -->
        <div class="pdf-pane">
          {#if pdfDoc}
            <div class="pdf-container">
              <div class="pdf-controls-top">
                <span>Page {pageNum} • {Math.round(scale * 100)}%</span>
              </div>
              <div class="pdf-canvas-container" bind:clientWidth={containerWidth}>
                <canvas bind:this={canvas} class="pdf-canvas"></canvas>
                {#if pdfLoading}
                  <div class="pdf-overlay">
                    <div class="loading-spinner"></div>
                    <p>Loading page {pageNum}...</p>
                  </div>
                {/if}
              </div>
            </div>
          {:else}
            <div class="loading-pane">
              <div class="loading-spinner"></div>
              <p>Loading PDF...</p>
            </div>
          {/if}
        </div>

        <!-- Notes & Excerpts Pane -->
        <div class="notes-pane">
          <div class="notes-header">
            <h3>Excerpts & Notes</h3>
            <div class="notes-actions">
              <button class="btn-small primary">+ New Excerpt</button>
              <button class="btn-small secondary">Review Cards</button>
            </div>
          </div>
          <div class="notes-content">
            {#if true}
              <div class="empty-notes">
                <div class="empty-icon">✨</div>
                <h4>Highlight text to create excerpts</h4>
                <p>Select any text in the PDF → it appears here → tap to make flashcard</p>
                <div class="keyboard-hints">
                  <span>← → Page navigation</span>
                  <span>+ − Zoom</span>
                  <span>Esc Back to library</span>
                </div>
              </div>
            {:else}
              <!-- Future: List of excerpts -->
            {/if}
          </div>
        </div>
      </div>
    </div>
  {/if}

  {#if message && !loading && !pdfLoading}
    <div class="toast" class:success={message.includes('successfully')}>
      {message}
    </div>
  {/if}
</main>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    background: #0a0a0f;
    color: #e2e8f0;
    font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    overflow-x: hidden;
  }

  :global(*, *::before, *::after) {
    box-sizing: border-box;
  }

  .app {
    min-height: 100vh;
    background: linear-gradient(135deg, #0a0a0f 0%, #1a1a2e 100%);
  }

  /* Library View */
  .library {
    padding: 2rem;
    max-width: 1400px;
    margin: 0 auto;
  }

  .header {
    text-align: center;
    margin-bottom: 4rem;
  }

  h1 {
    font-size: clamp(2.5rem, 6vw, 4.5rem);
    font-weight: 900;
    background: linear-gradient(135deg, #ff6b6b, #f7931e, #da1b60, #8b5cf6);
    background-size: 300% 300%;
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    animation: gradient-shift 8s ease infinite;
    margin: 0;
    letter-spacing: -0.02em;
  }

  @keyframes gradient-shift {
    0%, 100% { background-position: 0% 50%; }
    50% { background-position: 100% 50%; }
  }

  .tagline {
    font-size: 1.25rem;
    color: #94a3b8;
    margin-top: 1rem;
    font-weight: 400;
    max-width: 500px;
    margin-left: auto;
    margin-right: auto;
  }

  .toolbar {
    display: flex;
    justify-content: center;
    gap: 1.5rem;
    margin-bottom: 3rem;
    flex-wrap: wrap;
  }

  .btn-primary, .btn-secondary, .btn-small {
    padding: 1rem 2rem;
    border: none;
    border-radius: 16px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    font-size: 1rem;
    display: inline-flex;
    align-items: center;
    gap: 0.75rem;
    text-decoration: none;
    position: relative;
    overflow: hidden;
  }

  .btn-primary {
    background: linear-gradient(135deg, #da1b60, #c01852);
    color: white;
    box-shadow: 0 8px 32px rgba(218, 27, 96, 0.4);
  }

  .btn-primary:hover:not(:disabled) {
    transform: translateY(-3px);
    box-shadow: 0 16px 48px rgba(218, 27, 96, 0.6);
  }

  .btn-secondary, .btn-small {
    background: #252537;
    color: #cbd5e1;
    border: 1px solid #40414f;
  }

  .btn-secondary:hover:not(:disabled), .btn-small:hover:not(:disabled) {
    background: #2d2d42;
    border-color: #da1b60;
    color: white;
  }

  .btn-small {
    padding: 0.75rem 1.5rem;
    font-size: 0.875rem;
  }

  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    transform: none !important;
  }

  .spinner {
    width: 16px;
    height: 16px;
    border: 2px solid rgba(255,255,255,0.3);
    border-top: 2px solid white;
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }

  .empty-state {
    text-align: center;
    padding: 6rem 2rem;
    color: #64748b;
  }

  .empty-icon {
    font-size: 5rem;
    margin-bottom: 1.5rem;
    opacity: 0.7;
  }

  .empty-subtitle {
    font-size: 1.1rem;
    margin-top: 0.5rem;
    opacity: 0.8;
  }

  .library-stats {
    text-align: center;
    margin: 2rem 0 3rem;
  }

  .stat {
    display: inline-block;
    background: rgba(218, 27, 96, 0.2);
    color: #fda4af;
    padding: 0.5rem 1.25rem;
    border-radius: 50px;
    font-size: 0.9rem;
    font-weight: 500;
    margin: 0 0.75rem;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(380px, 1fr));
    gap: 2rem;
    margin-top: 2rem;
  }

  .card {
    background: linear-gradient(145deg, #1a1a24, #161623);
    border-radius: 24px;
    overflow: hidden;
    box-shadow: 
      0 20px 40px rgba(0,0,0,0.4),
      0 1px 0 rgba(255,255,255,0.1) inset;
    transition: all 0.4s cubic-bezier(0.4, 0, 0.2, 1);
    cursor: pointer;
    border: 1px solid rgba(255,255,255,0.05);
    height: 160px;
    display: flex;
    flex-direction: column;
  }

  .card:hover {
    transform: translateY(-12px) scale(1.02);
    box-shadow: 
      0 32px 64px rgba(218, 27, 96, 0.25),
      0 1px 0 rgba(255,255,255,0.15) inset;
    border-color: rgba(218, 27, 96, 0.3);
  }

  .card:focus {
    outline: 3px solid rgba(218, 27, 96, 0.5);
    outline-offset: 2px;
  }

  .card-content {
    padding: 2rem;
    flex-grow: 1;
    display: flex;
    flex-direction: column;
  }

  .title {
    font-size: 1.4rem;
    font-weight: 700;
    margin-bottom: 0.75rem;
    color: #f8fafc;
    line-height: 1.3;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .meta {
    font-size: 0.95rem;
    color: #94a3b8;
    margin-top: auto;
  }

  .open-btn {
    background: linear-gradient(135deg, #da1b60, #c01852);
    color: white;
    border: none;
    padding: 1.25rem;
    font-weight: 700;
    font-size: 0.95rem;
    cursor: pointer;
    transition: all 0.3s ease;
    margin-top: auto;
  }

  .open-btn:hover {
    background: linear-gradient(135deg, #c01852, #a11244);
    transform: translateY(-2px);
  }

  /* Reader View */
  .reader {
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: #0f0f17;
  }

  .reader-header {
    padding: 1.25rem 2rem;
    background: rgba(20, 20, 28, 0.95);
    backdrop-filter: blur(20px);
    border-bottom: 1px solid rgba(65, 65, 79, 0.5);
    display: flex;
    align-items: center;
    gap: 1.5rem;
    position: sticky;
    top: 0;
    z-index: 100;
  }

  .back-btn {
    background: rgba(37, 37, 55, 0.8);
    border: 1px solid rgba(65, 65, 79, 0.5);
    color: #cbd5e1;
    font-size: 0.95rem;
    padding: 0.75rem 1.5rem;
    border-radius: 12px;
    cursor: pointer;
    transition: all 0.3s ease;
    font-weight: 500;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .back-btn:hover {
    background: #da1b60;
    border-color: #da1b60;
    color: white;
    transform: translateX(-2px);
  }

  .paper-info {
    flex: 1;
    min-width: 0;
  }

  .paper-info h2 {
    margin: 0 0 0.25rem 0;
    font-size: 1.5rem;
    font-weight: 700;
    color: #f8fafc;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .page-counter {
    color: #94a3b8;
    font-size: 0.875rem;
    font-weight: 500;
  }

  .pdf-controls {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .pdf-controls button {
    background: rgba(37, 37, 55, 0.8);
    color: #cbd5e1;
    border: 1px solid rgba(65, 65, 79, 0.5);
    width: 42px;
    height: 42px;
    border-radius: 10px;
    cursor: pointer;
    font-size: 1.1rem;
    font-weight: 600;
    transition: all 0.2s ease;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .pdf-controls button:hover:not(:disabled) {
    background: #da1b60;
    border-color: #da1b60;
    color: white;
    transform: scale(1.05);
  }

  .zoom-level {
    min-width: 50px;
    text-align: center;
    font-weight: 600;
    color: #f8fafc;
    font-size: 0.95rem;
    font-variant-numeric: tabular-nums;
  }

  .split-view {
    display: flex;
    flex: 1;
    overflow: hidden;
    background: #0a0a0f;
  }

  .pdf-pane {
    flex: 6;
    position: relative;
    background: #000;
    display: flex;
    flex-direction: column;
  }

  .pdf-container {
    flex: 1;
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .pdf-controls-top {
    padding: 0.75rem 1.5rem;
    background: rgba(0,0,0,0.8);
    backdrop-filter: blur(10px);
    border-bottom: 1px solid rgba(65,65,79,0.3);
    font-size: 0.875rem;
    color: #94a3b8;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .pdf-canvas-container {
    flex: 1;
    overflow: auto;
    background: #000;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding: 2rem;
    position: relative;
  }

  .pdf-canvas {
    max-width: 100%;
    max-height: 100%;
    box-shadow: 0 20px 60px rgba(0,0,0,0.8);
    border-radius: 8px;
    cursor: grab;
  }

  .pdf-canvas:active {
    cursor: grabbing;
  }

  .pdf-overlay {
    position: absolute;
    inset: 0;
    background: rgba(0,0,0,0.9);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 1.5rem;
    z-index: 10;
  }

  .loading-spinner {
    width: 48px;
    height: 48px;
    border: 4px solid rgba(218, 27, 96, 0.2);
    border-top: 4px solid #da1b60;
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  .loading-pane {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: #64748b;
    font-size: 1.25rem;
    gap: 1rem;
  }

  .notes-pane {
    flex: 4;
    background: linear-gradient(180deg, #1a1a24 0%, #161623 100%);
    border-left: 1px solid rgba(65, 65, 79, 0.5);
    display: flex;
    flex-direction: column;
  }

  .notes-header {
    padding: 1.75rem 2rem;
    border-bottom: 1px solid rgba(65, 65, 79, 0.3);
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .notes-header h3 {
    margin: 0;
    font-size: 1.4rem;
    font-weight: 700;
    background: linear-gradient(135deg, #f8fafc, #cbd5e1);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
  }

  .notes-actions {
    display: flex;
    gap: 1rem;
  }

  .notes-content {
    flex: 1;
    overflow: auto;
    padding: 2rem;
  }

  .empty-notes {
    text-align: center;
    padding: 3rem 2rem;
    color: #94a3b8;
  }

  .empty-notes .empty-icon {
    font-size: 3.5rem;
    margin-bottom: 1.5rem;
    opacity: 0.7;
  }

  .empty-notes h4 {
    font-size: 1.4rem;
    margin: 0 0 1rem 0;
    font-weight: 600;
    color: #f1f5f9;
  }

  .keyboard-hints {
    margin-top: 2rem;
    display: flex;
    flex-wrap: wrap;
    gap: 1.5rem;
    justify-content: center;
    font-size: 0.875rem;
  }

  .keyboard-hints span {
    background: rgba(218, 27, 96, 0.15);
    color: #fda4af;
    padding: 0.5rem 1rem;
    border-radius: 8px;
    font-weight: 500;
    border: 1px solid rgba(218, 27, 96, 0.3);
  }

  .toast {
    position: fixed;
    bottom: 2.5rem;
    left: 50%;
    transform: translateX(-50%);
    background: linear-gradient(135deg, #da1b60, #c01852);
    color: white;
    padding: 1.25rem 2.5rem;
    border-radius: 20px;
    box-shadow: 0 20px 60px rgba(218, 27, 96, 0.5);
    font-weight: 500;
    max-width: 500px;
    animation: toastSlide 0.4s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .toast.success {
    background: linear-gradient(135deg, #10b981, #059669);
    box-shadow: 0 20px 60px rgba(16, 185, 129, 0.4);
  }

  @keyframes toastSlide {
    from {
      opacity: 0;
      transform: translateX(-50%) translateY(20px) scale(0.95);
    }
    to {
      opacity: 1;
      transform: translateX(-50%) translateY(0) scale(1);
    }
  }

  /* Responsive */
  @media (max-width: 1024px) {
    .split-view {
      flex-direction: column;
    }
    
    .pdf-pane, .notes-pane {
      flex: none;
      height: 50vh;
    }
    
    .pdf-canvas-container {
      padding: 1rem;
    }
  }

  @media (max-width: 768px) {
    .grid {
      grid-template-columns: 1fr;
      gap: 1.5rem;
    }
    
    .toolbar {
      flex-direction: column;
      align-items: center;
    }
    
    .reader-header {
      flex-direction: column;
      gap: 1rem;
      padding: 1rem;
      text-align: center;
    }
    
    .pdf-controls {
      order: -1;
      width: 100%;
      justify-content: center;
    }
  }
</style>
