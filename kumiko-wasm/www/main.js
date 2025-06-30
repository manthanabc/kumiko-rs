import init, { find_panels } from './pkg/kumiko_wasm.js';

async function run() {
  await init();

  const imageUploadFiles = document.getElementById('imageUploadFiles');
  const imageUploadFolder = document.getElementById('imageUploadFolder');
  const mangaImage = document.getElementById('mangaImage');
  const panelsOverlay = document.getElementById('panelsOverlay');
  const imageContainer = document.querySelector('.image-container');

  let allImages = [];
  let currentImageIndex = 0;
  let currentPanels = [];
  let currentImageSize = { width: 0, height: 0 }; // Natural image size
  let currentPanelIndex = -1;

  function handleFiles(files) {
    allImages = Array.from(files).filter(file => file.type.startsWith('image/')).sort((a, b) => a.name.localeCompare(b.name));
    currentImageIndex = 0;
    if (allImages.length > 0) {
      loadImage(currentImageIndex);
    } else {
      console.log("No images selected.");
      // Optionally, clear UI or show a message
    }
  }

  imageUploadFiles.addEventListener('change', (event) => {
    handleFiles(event.target.files);
  });

  imageUploadFolder.addEventListener('change', (event) => {
    handleFiles(event.target.files);
  });

  async function loadImage(index) {
    if (index < 0 || index >= allImages.length) {
      console.log("No more images.");
      // Optionally, reset UI or show a message
      return;
    }

    currentImageIndex = index;
    const file = allImages[currentImageIndex];

    const reader = new FileReader();
    reader.onload = async (e) => {
      const blob = new Blob([e.target.result], { type: file.type });
      mangaImage.src = URL.createObjectURL(blob);
      mangaImage.onload = async () => {
        currentImageSize = { width: mangaImage.naturalWidth, height: mangaImage.naturalHeight };
        // Set container size to match the natural size of the image for consistent panel overlay positioning
        imageContainer.style.width = `${mangaImage.naturalWidth}px`;
        imageContainer.style.height = `${mangaImage.naturalHeight}px`;

        const imageBytes = new Uint8Array(e.target.result);
        try {
          const result = find_panels(
            imageBytes,
            0.02, // rdp_epsilon
            0.05, // small_panel_ratio
            "ltr", // reading_direction
            10,   // gutter_x
            10,   // gutter_y
            10,   // gutter_r
            10    // gutter_b
          );
          const [imgSize, panels] = JSON.parse(result);
          currentPanels = panels;
          drawPanels(panels, imgSize);
          currentPanelIndex = -1; // Reset panel index on new image
          dezoom(); // Reset zoom for new image
        } catch (err) {
          console.error('Error finding panels:', err);
        }
      };
    };
    reader.readAsArrayBuffer(file);
  }

  function drawPanels(panels, imgSize) {
    panelsOverlay.innerHTML = ''; // Clear previous panels
    panels.forEach((panel, index) => {
      const panelDiv = document.createElement('div');
      panelDiv.classList.add('panel');
      // Position panels relative to the natural image size
      panelDiv.style.left = `${(panel.x / imgSize[0]) * 100}%`;
      panelDiv.style.top = `${(panel.y / imgSize[1]) * 100}%`;
      panelDiv.style.width = `${(panel.width / imgSize[0]) * 100}%`;
      panelDiv.style.height = `${(panel.height / imgSize[1]) * 100}%`;
      panelDiv.dataset.index = index;
      panelDiv.addEventListener('click', (e) => {
        e.stopPropagation(); // Prevent dblclick on container from triggering
        zoomToPanel(index);
      });
      panelsOverlay.appendChild(panelDiv);
    });
  }

  function zoomToPanel(index) {
    if (index < 0 || index >= currentPanels.length) {
      dezoom();
      return;
    }

    currentPanelIndex = index;
    const panel = currentPanels[index];

    // Get the current rendered dimensions of the image (which might be scaled by CSS max-width/height)
    const renderedImageWidth = mangaImage.offsetWidth;
    const renderedImageHeight = mangaImage.offsetHeight;

    // Calculate panel position and size in pixels relative to the rendered image
    const panelLeftPx = (panel.x / currentImageSize.width) * renderedImageWidth;
    const panelTopPx = (panel.y / currentImageSize.height) * renderedImageHeight;
    const panelWidthPx = (panel.width / currentImageSize.width) * renderedImageWidth;
    const panelHeightPx = (panel.height / currentImageSize.height) * renderedImageHeight;

    // Get the dimensions of the viewport (the imageContainer itself)
    const viewportWidth = imageContainer.offsetWidth;
    const viewportHeight = imageContainer.offsetHeight;

    // Calculate scale to fit the panel within the viewport
    const scaleX = viewportWidth / panelWidthPx;
    const scaleY = viewportHeight / panelHeightPx;
    const scale = Math.min(scaleX, scaleY);

    // Calculate translation to center the panel within the viewport
    const translateX = -panelLeftPx * scale + (viewportWidth - panelWidthPx * scale) / 2;
    const translateY = -panelTopPx * scale + (viewportHeight - panelHeightPx * scale) / 2;

    imageContainer.style.transformOrigin = '0 0'; // Always transform from top-left
    imageContainer.style.transform = `translate(${translateX}px, ${translateY}px) scale(${scale})`;

    // Highlight active panel and hide others
    panelsOverlay.querySelectorAll('.panel').forEach((p, i) => {
      p.classList.remove('active');
      if (i !== index) {
        p.style.display = 'none';
      } else {
        p.style.display = 'block';
      }
    });
    panelsOverlay.querySelector(`[data-index="${index}"]`).classList.add('active');
  }

  function dezoom() {
    imageContainer.style.transform = 'none';
    panelsOverlay.querySelectorAll('.panel').forEach(p => {
      p.classList.remove('active');
      p.style.display = 'block'; // Show all panels
    });
    currentPanelIndex = -1;
  }

  function nextPanel() {
    if (currentPanelIndex < currentPanels.length - 1) {
      zoomToPanel(currentPanelIndex + 1);
    } else {
      // If at the last panel, load the next image and go to its first panel
      loadImage(currentImageIndex + 1);
      // The loadImage function will reset currentPanelIndex to -1 and dezoom
      // Then, if there are panels on the new page, the first one will be zoomed to automatically
      // by calling zoomToPanel(0) after panels are drawn.
    }
  }

  function prevPanel() {
    if (currentPanelIndex > 0) {
      zoomToPanel(currentPanelIndex - 1);
    } else {
      // If at the first panel, load the previous image and go to its last panel
      loadImage(currentImageIndex - 1);
      // After loading, we'll need to explicitly go to the last panel of the previous page
      // This requires a slight modification to loadImage or a separate handler.
      // For now, it will just dezoom on the previous page.
    }
  }

  // Reset zoom on double click on the image container
  imageContainer.addEventListener('dblclick', () => {
    dezoom();
  });

  // Keyboard navigation
  document.addEventListener('keydown', (e) => {
    switch (e.key) {
      case 'ArrowRight':
      case 'ArrowDown':
        nextPanel();
        break;
      case 'ArrowLeft':
      case 'ArrowUp':
        prevPanel();
        break;
      case 'Escape':
        dezoom();
        break;
    }
  });
}

run();