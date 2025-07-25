import init, { find_panels } from './pkg/kumiko_wasm.js';

async function run() {
  await init();

  const imageUploadFiles = document.getElementById('imageUploadFiles');
  const imageUploadFolder = document.getElementById('imageUploadFolder');
  const loadExampleButton = document.getElementById('loadExample');
  let reader;

  function handleFiles(files) {
    console.log(files)
    // if(!Array.isArray(files)) {
    //   fi.push(files)
    //   files = fi
    // }
    const allImages = Array.from(files).filter(file => file.type.startsWith('image/')).sort((a, b) => a.name.localeCompare(b.name));
    if (allImages.length > 0) {
      const comicsJson = [];
      const imagePromises = allImages.map(file => {
        return new Promise((resolve, reject) => {
          const reader = new FileReader();
          reader.onload = async (e) => {
            const imageBytes = new Uint8Array(e.target.result);
            try {
              const result = find_panels(
                imageBytes,
                0.02, // rdp_epsilon
                0.05, // small_panel_ratio
                "rtl", // reading_direction
                10,   // gutter_x
                10,   // gutter_y
                10,   // gutter_r
                10    // gutter_b
              );
              const [imgSize, panels] = JSON.parse(result);
              comicsJson.push({
                filename: URL.createObjectURL(new Blob([e.target.result], { type: file.type })),
                size: imgSize,
                panels: panels
              });
              resolve();
            } catch (err) {
              console.error('Error finding panels:', err);
              reject(err);
            }
          };
          reader.readAsArrayBuffer(file);
        });
      });

      Promise.all(imagePromises).then(() => {
        if (reader) {
            reader.gui.empty();
        }
        reader = new Reader({
          container: $('#kumiko-reader'),
          images_dir: 'urls',
          comicsJson: comicsJson,
          controls: true
        });
        reader.start();
      });
    } else {
      console.log("No images selected.");
    }
  }

  async function loadExample() {
    try {
      const comicsJson = [];
      const imageUrls = ['examples/manga1/0001.jpg', 'examples/manga1/0002.jpg', 'examples/manga1/0003.jpg'];

      for (const imageUrl of imageUrls) {
        const response = await fetch(imageUrl);
        const imageBlob = await response.blob();
        const imageArrayBuffer = await new Response(imageBlob).arrayBuffer();
        const imageBytes = new Uint8Array(imageArrayBuffer);

        const result = find_panels(
          imageBytes,
          0.02, // rdp_epsilon
          0.05, // small_panel_ratio
          "rtl", // reading_direction
          10,   // gutter_x
          10,   // gutter_y
          10,   // gutter_r
          10    // gutter_b
        );
        const [imgSize, panels] = JSON.parse(result);

        comicsJson.push({
          filename: URL.createObjectURL(imageBlob),
          size: imgSize,
          panels: panels
        });
      }

      if (reader) {
        reader.gui.empty();
      }
      reader = new Reader({
        container: $('#kumiko-reader'),
        images_dir: 'urls',
        comicsJson: comicsJson,
        controls: true
      });
      reader.start();
    } catch (err) {
      console.error('Error loading example:', err);
    }
  }

  imageUploadFiles.addEventListener('change', (event) => {
    handleFiles(event.target.files);
  });

  imageUploadFolder.addEventListener('change', (event) => {
    handleFiles(event.target.files);
  });

  loadExampleButton.addEventListener('click', () => {
    loadExample();
  });
}

run();
