import './style.css';
import { Chart, registerables } from 'chart.js';
import { 
  createIcons, Folder, File, Zap, Cpu, Shield, Layers, 
  Image, HardDrive, Download, ChevronRight, ChevronDown, 
  Trash2, BarChart2, Eye, Copy, ExternalLink, Database 
} from 'lucide';

// Register all Chart.js components
Chart.register(...registerables);

// --- HELPER: Human Readable Bytes ---
function formatBytes(bytes) {
  if (bytes === 0) return '0 Bytes';
  const k = 1024;
  const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

// --- MOCK FILESYSTEM DATA ---
const mockData = {
  name: "Root",
  type: "directory",
  sizeBytes: 45200000000,
  path: "/",
  children: [
    {
      name: "Games",
      type: "directory",
      sizeBytes: 24500000000,
      path: "/Games",
      children: [
        {
          name: "Cyberpunk2077",
          type: "directory",
          sizeBytes: 18200000000,
          path: "/Games/Cyberpunk2077",
          children: [
            { name: "archive.archive", type: "file", ext: "archive", sizeBytes: 17500000000, path: "/Games/Cyberpunk2077/archive.archive" },
            { name: "cyberpunk.exe", type: "file", ext: "code", sizeBytes: 580000000, path: "/Games/Cyberpunk2077/cyberpunk.exe" },
            { name: "readme.txt", type: "file", ext: "other", sizeBytes: 120000000, path: "/Games/Cyberpunk2077/readme.txt" }
          ]
        },
        {
          name: "SteamLibrary",
          type: "directory",
          sizeBytes: 6300000000,
          path: "/Games/SteamLibrary",
          children: [
            { name: "common_data.bin", type: "file", ext: "archive", sizeBytes: 6100000000, path: "/Games/SteamLibrary/common_data.bin" },
            { name: "steam.ico", type: "file", ext: "image", sizeBytes: 200000000, path: "/Games/SteamLibrary/steam.ico" }
          ]
        }
      ]
    },
    {
      name: "Videos",
      type: "directory",
      sizeBytes: 12400000000,
      path: "/Videos",
      children: [
        { name: "render_final.mp4", type: "file", ext: "video", sizeBytes: 8400000000, path: "/Videos/render_final.mp4" },
        { name: "stream_record.mp4", type: "file", ext: "video", sizeBytes: 3900000000, path: "/Videos/stream_record.mp4" },
        { name: "intro_logo.mov", type: "file", ext: "video", sizeBytes: 100000000, path: "/Videos/intro_logo.mov" }
      ]
    },
    {
      name: "Projects",
      type: "directory",
      sizeBytes: 4800000000,
      path: "/Projects",
      children: [
        {
          name: "rust-compiler",
          type: "directory",
          sizeBytes: 3200000000,
          path: "/Projects/rust-compiler",
          children: [
            { name: "libstd.so", type: "file", ext: "code", sizeBytes: 3000000000, path: "/Projects/rust-compiler/libstd.so" },
            { name: "main.rs", type: "file", ext: "code", sizeBytes: 200000000, path: "/Projects/rust-compiler/main.rs" }
          ]
        },
        {
          name: "edirstat",
          type: "directory",
          sizeBytes: 1600000000,
          path: "/Projects/edirstat",
          children: [
            { name: "edirstat-binary", type: "file", ext: "code", sizeBytes: 1400000000, path: "/Projects/edirstat/edirstat-binary" },
            { name: "arena.rs", type: "file", ext: "code", sizeBytes: 200000000, path: "/Projects/edirstat/arena.rs" }
          ]
        }
      ]
    },
    {
      name: "Downloads",
      type: "directory",
      sizeBytes: 3500000000,
      path: "/Downloads",
      children: [
        { name: "ubuntu-26.04-desktop.iso", type: "file", ext: "archive", sizeBytes: 2800000000, path: "/Downloads/ubuntu-26.04-desktop.iso" },
        { name: "wallpaper.png", type: "file", ext: "image", sizeBytes: 450000000, path: "/Downloads/wallpaper.png" },
        { name: "music_album.zip", type: "file", ext: "archive", sizeBytes: 250000000, path: "/Downloads/music_album.zip" }
      ]
    }
  ]
};

// --- SIMULATOR: Left Panel Tree Explorer ---
function createTreeDOM(node, depth = 0) {
  const nodeDiv = document.createElement('div');
  nodeDiv.className = 'tree-node';
  nodeDiv.setAttribute('data-path', node.path);
  
  const rowDiv = document.createElement('div');
  rowDiv.className = 'tree-node-row';
  rowDiv.style.paddingLeft = `${(depth * 14) + 8}px`;
  
  const leftSide = document.createElement('div');
  leftSide.className = 'tree-node-left';
  
  // Icon
  const icon = document.createElement('i');
  icon.className = 'tree-icon';
  if (node.type === 'directory') {
    icon.setAttribute('data-lucide', 'folder');
    icon.classList.add('tree-folder-icon');
  } else {
    icon.setAttribute('data-lucide', 'file');
    icon.classList.add('tree-file-icon');
  }
  leftSide.appendChild(icon);
  
  // Name
  const nameSpan = document.createElement('span');
  nameSpan.className = 'tree-node-name';
  nameSpan.textContent = node.name;
  leftSide.appendChild(nameSpan);
  
  rowDiv.appendChild(leftSide);
  
  // Size
  const sizeSpan = document.createElement('span');
  sizeSpan.className = 'tree-node-size';
  sizeSpan.textContent = formatBytes(node.sizeBytes);
  rowDiv.appendChild(sizeSpan);
  
  nodeDiv.appendChild(rowDiv);
  
  // Children
  if (node.type === 'directory' && node.children) {
    const childrenContainer = document.createElement('div');
    childrenContainer.className = 'tree-node-children';
    node.children.forEach(child => {
      childrenContainer.appendChild(createTreeDOM(child, depth + 1));
    });
    nodeDiv.appendChild(childrenContainer);
  }
  
  // Event listeners for tree row selection
  rowDiv.addEventListener('click', (e) => {
    e.stopPropagation();
    selectNode(node.path);
  });
  
  rowDiv.addEventListener('mouseenter', () => {
    highlightBlock(node.path);
    updateFooter(node.path, node.sizeBytes);
  });
  
  rowDiv.addEventListener('mouseleave', () => {
    removeBlockHighlight(node.path);
    resetFooterToSelected();
  });
  
  return nodeDiv;
}

// --- SIMULATOR: Right Panel Treemap Generator (Squarified Treemap) ---
// Aspect ratio helper
function worst(rowAreas, L) {
  if (rowAreas.length === 0) return Infinity;
  const sum = rowAreas.reduce((s, val) => s + val, 0);
  if (sum === 0) return Infinity;
  let maxRatio = 0;
  for (let val of rowAreas) {
    const ratio = Math.max((val * L * L) / (sum * sum), (sum * sum) / (val * L * L));
    if (ratio > maxRatio) maxRatio = ratio;
  }
  return maxRatio;
}

function layoutRow(row, L, x, y, w, h, vertical, canvas) {
  const sum = row.reduce((s, n) => s + n.area, 0);
  if (sum === 0) return;
  
  if (vertical) {
    const rowW = sum / h;
    let currentY = y;
    row.forEach(node => {
      const nodeH = node.area / rowW;
      renderNodeOrRecurse(node.node, canvas, x, currentY, rowW, nodeH);
      currentY += nodeH;
    });
  } else {
    const rowH = sum / w;
    let currentX = x;
    row.forEach(node => {
      const nodeW = node.area / rowH;
      renderNodeOrRecurse(node.node, canvas, currentX, y, nodeW, rowH);
      currentX += nodeW;
    });
  }
}

function squarify(nodes, x, y, w, h, canvas) {
  if (nodes.length === 0) return;
  
  const totalSize = nodes.reduce((sum, n) => sum + n.sizeBytes, 0);
  if (totalSize === 0) return;
  
  const scale = (w * h) / totalSize;
  const sortedNodes = nodes
    .map(node => ({ node, area: node.sizeBytes * scale }))
    .sort((a, b) => b.area - a.area);
    
  let currentX = x;
  let currentY = y;
  let currentW = w;
  let currentH = h;
  
  let row = [];
  let index = 0;
  
  while (index < sortedNodes.length) {
    const nextNode = sortedNodes[index];
    if (currentW <= 0.001 || currentH <= 0.001) {
      row.push(nextNode);
      index++;
      continue;
    }
    
    const L = Math.min(currentW, currentH);
    const rowAreas = row.map(r => r.area);
    const currentWorst = worst(rowAreas, L);
    const nextWorst = worst([...rowAreas, nextNode.area], L);
    
    if (nextWorst <= currentWorst) {
      row.push(nextNode);
      index++;
    } else {
      const rowAreaSum = row.reduce((s, r) => s + r.area, 0);
      const isVertical = currentH <= currentW;
      
      layoutRow(row, L, currentX, currentY, currentW, currentH, isVertical, canvas);
      
      if (isVertical) {
        const rowW = rowAreaSum / currentH;
        currentX += rowW;
        currentW -= rowW;
      } else {
        const rowH = rowAreaSum / currentW;
        currentY += rowH;
        currentH -= rowH;
      }
      
      row = [];
    }
  }
  
  if (row.length > 0) {
    const rowAreaSum = row.reduce((s, r) => s + r.area, 0);
    const isVertical = currentH <= currentW;
    layoutRow(row, Math.min(currentW, currentH), currentX, currentY, currentW, currentH, isVertical, canvas);
  }
}

function renderNodeOrRecurse(node, canvas, x, y, w, h) {
  w = Math.max(w, 0.01);
  h = Math.max(h, 0.01);

  if (node.type === 'file') {
    const block = document.createElement('div');
    block.className = `tm-node ext-${node.ext || 'other'}`;
    block.style.left = `${x}%`;
    block.style.top = `${y}%`;
    block.style.width = `${w}%`;
    block.style.height = `${h}%`;
    block.setAttribute('data-path', node.path);
    block.setAttribute('data-size', node.sizeBytes);
    block.setAttribute('data-name', node.name);
    
    block.addEventListener('mouseenter', () => {
      highlightTreeNode(node.path);
      updateFooter(node.path, node.sizeBytes);
    });
    block.addEventListener('mouseleave', () => {
      removeTreeHighlight(node.path);
      resetFooterToSelected();
    });
    block.addEventListener('click', (e) => {
      e.stopPropagation();
      selectNode(node.path);
    });
    
    canvas.appendChild(block);
  } else if (node.type === 'directory' && node.children) {
    squarify(node.children, x, y, w, h, canvas);
  }
}

function renderTreemap(node, canvas, x = 0, y = 0, w = 100, h = 100, isHorizontal = true) {
  renderNodeOrRecurse(node, canvas, x, y, w, h);
}

// --- SELECTING & HIGHLIGHTING COORDINATOR ---
let selectedPath = null;

function selectNode(path) {
  if (selectedPath === path) {
    // Clicked on already selected path -> deselect
    selectedPath = null;
    document.querySelectorAll('.tree-node').forEach(el => el.classList.remove('selected'));
    document.querySelectorAll('.tm-node').forEach(el => el.classList.remove('selected-block'));
    resetFooterToSelected();
    return;
  }

  selectedPath = path;
  
  // Reset previous selections
  document.querySelectorAll('.tree-node').forEach(el => el.classList.remove('selected'));
  document.querySelectorAll('.tm-node').forEach(el => el.classList.remove('selected-block'));
  
  // Select in tree
  const treeEl = document.querySelector(`.tree-node[data-path="${path}"]`);
  if (treeEl) {
    treeEl.classList.add('selected');
    // Scroll into view if needed inside explorer
    treeEl.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
  }
  
  // Select in treemap (leaf or parent group)
  const mapEl = document.querySelector(`.tm-node[data-path="${path}"]`);
  if (mapEl) {
    mapEl.classList.add('selected-block');
  } else {
    // If it's a directory, highlight all its children blocks
    document.querySelectorAll(`.tm-node[data-path^="${path === '/' ? '/' : path + '/'}"]`).forEach(el => {
      el.classList.add('selected-block');
    });
  }
  
  // Update footer permanently
  const node = findNodeByPath(mockData, path);
  if (node) {
    updateFooter(node.path, node.sizeBytes, true);
  }
}

function highlightBlock(path) {
  const mapEl = document.querySelector(`.tm-node[data-path="${path}"]`);
  if (mapEl) {
    mapEl.classList.add('highlighted');
  } else {
    // Directory - highlight descendant blocks
    document.querySelectorAll(`.tm-node[data-path^="${path === '/' ? '/' : path + '/'}"]`).forEach(el => {
      el.classList.add('highlighted');
    });
  }
}

function removeBlockHighlight(path) {
  document.querySelectorAll('.tm-node').forEach(el => el.classList.remove('highlighted'));
}

function highlightTreeNode(path) {
  const treeEl = document.querySelector(`.tree-node[data-path="${path}"]`);
  if (treeEl) {
    treeEl.querySelector('.tree-node-row').classList.add('hovered');
  }
}

function removeTreeHighlight(path) {
  document.querySelectorAll('.tree-node-row').forEach(el => el.classList.remove('hovered'));
}

function updateFooter(path, size, isPermanent = false) {
  const pathEl = document.getElementById('sim-footer-path');
  const sizeEl = document.getElementById('sim-footer-size');
  const iconEl = document.getElementById('sim-footer-icon');
  
  pathEl.textContent = path;
  pathEl.classList.add('active');
  sizeEl.textContent = formatBytes(size);
  
  // Dynamically change icon based on file type
  const node = findNodeByPath(mockData, path);
  if (node && node.type === 'directory') {
    iconEl.setAttribute('data-lucide', 'folder');
  } else {
    iconEl.setAttribute('data-lucide', 'file');
  }
  createIcons({ icons: { Folder, File } });
}

function resetFooterToSelected() {
  if (selectedPath) {
    const node = findNodeByPath(mockData, selectedPath);
    if (node) {
      updateFooter(node.path, node.sizeBytes);
      return;
    }
  }
  
  const pathEl = document.getElementById('sim-footer-path');
  const sizeEl = document.getElementById('sim-footer-size');
  pathEl.textContent = "Hover or click a block above to inspect...";
  pathEl.classList.remove('active');
  sizeEl.textContent = "";
}

function findNodeByPath(node, path) {
  if (node.path === path) return node;
  if (node.children) {
    for (let child of node.children) {
      const result = findNodeByPath(child, path);
      if (result) return result;
    }
  }
  return null;
}

// --- BENCHMARKS: CHART.JS SETUP ---
const benchmarkData = {
  nvme: {
    title: "Samsung 990 Pro NVMe SSD (Gen 4)",
    desc: "Scanning dense repositories containing millions of files and nested directories. (Warm Cache)",
    labels: ['eDirStat (Rust, Parallel)', 'QDirStat (Perl Backend)', 'WinDirStat (Legacy C++)', 'WizTree (Windows MFT)'],
    dnfTexts: [null, null, "Incompatible (Not supported on Linux/btrfs)", "Incompatible (Not supported on Linux/btrfs)"],
    datasets: [{
      label: 'Median Scan Duration (Seconds)',
      data: [0.86, 6.91, null, null],
      backgroundColor: [
        'rgba(124, 58, 237, 0.85)', // primary violet glow
        'rgba(6, 182, 212, 0.6)',  // cyan
        'rgba(148, 163, 184, 0.4)', // slate
        'rgba(236, 72, 153, 0.4)'  // pink
      ],
      borderColor: [
        '#c084fc',
        '#22d3ee',
        '#cbd5e1',
        '#f472b6'
      ],
      borderWidth: 1.5,
      borderRadius: 6,
      barThickness: 40
    }]
  },
  sata: {
    title: "Samsung SSD 870 QVO (8TB SATA)",
    desc: "Scanning game installations containing a mix of large zip archives and small asset files.",
    labels: ['eDirStat (Rust, Parallel)', 'QDirStat (Perl Backend)', 'WinDirStat (Legacy C++)', 'WizTree (Windows MFT)'],
    dnfTexts: [null, null, "Incompatible (Not supported on Linux/btrfs)", "Incompatible (Not supported on Linux/btrfs)"],
    datasets: [{
      label: 'Median Scan Duration (Seconds)',
      data: [0.47, 4.54, null, null],
      backgroundColor: [
        'rgba(124, 58, 237, 0.85)',
        'rgba(6, 182, 212, 0.6)',
        'rgba(148, 163, 184, 0.4)',
        'rgba(236, 72, 153, 0.4)'
      ],
      borderColor: [
        '#c084fc',
        '#22d3ee',
        '#cbd5e1',
        '#f472b6'
      ],
      borderWidth: 1.5,
      borderRadius: 6,
      barThickness: 40
    }]
  },
  hdd: {
    title: "Toshiba MG09SACA 16TB Mechanical HDD",
    desc: "Traversing massive deeply nested directory structures on traditional spinning disks.",
    labels: ['eDirStat (Rust, Parallel)', 'QDirStat (Perl Backend)', 'WinDirStat (Legacy C++)', 'WizTree (Windows MFT)'],
    dnfTexts: [null, null, "Incompatible (Not supported on Linux/btrfs)", "Incompatible (Not supported on Linux/btrfs)"],
    datasets: [{
      label: 'Median Scan Duration (Seconds)',
      data: [0.53, 3.54, null, null],
      backgroundColor: [
        'rgba(124, 58, 237, 0.85)',
        'rgba(6, 182, 212, 0.6)',
        'rgba(148, 163, 184, 0.4)',
        'rgba(236, 72, 153, 0.4)'
      ],
      borderColor: [
        '#c084fc',
        '#22d3ee',
        '#cbd5e1',
        '#f472b6'
      ],
      borderWidth: 1.5,
      borderRadius: 6,
      barThickness: 40
    }]
  },
  mzvlb: {
    title: "SAMSUNG MZVLB512HBJQ PCIe SSD",
    desc: "Scanning Windows system directories containing deep system libraries and DLLs.",
    labels: ['eDirStat (Rust, Parallel)', 'WizTree (Windows MFT)', 'WinDirStat (Legacy C++)', 'QDirStat (Perl Backend)'],
    dnfTexts: [null, null, null, "Incompatible (Not supported on Windows)"],
    datasets: [{
      label: 'Median Scan Duration (Seconds)',
      data: [1.72, 4.41, 92.38, null],
      backgroundColor: [
        'rgba(124, 58, 237, 0.85)',
        'rgba(236, 72, 153, 0.6)',
        'rgba(148, 163, 184, 0.4)',
        'rgba(6, 182, 212, 0.4)'
      ],
      borderColor: [
        '#c084fc',
        '#f472b6',
        '#cbd5e1',
        '#22d3ee'
      ],
      borderWidth: 1.5,
      borderRadius: 6,
      barThickness: 40
    }]
  }
};

let benchmarkChart = null;

function initChart() {
  const ctx = document.getElementById('benchmarkChart').getContext('2d');
  
  benchmarkChart = new Chart(ctx, {
    type: 'bar',
    data: JSON.parse(JSON.stringify(benchmarkData.mzvlb)), // Clone
    plugins: [{
      id: 'dnfPlugin',
      afterDatasetsDraw(chart) {
        const { ctx, chartArea: { left }, scales: { y } } = chart;
        ctx.save();
        chart.data.datasets.forEach((dataset, datasetIndex) => {
          const meta = chart.getDatasetMeta(datasetIndex);
          meta.data.forEach((bar, index) => {
            const val = dataset.data[index];
            if (val === null || val === undefined || isNaN(val)) {
              const text = chart.data.dnfTexts?.[index] || 'Incompatible';
              ctx.font = 'bold 12px "JetBrains Mono", monospace';
              ctx.fillStyle = '#ef4444'; // Red
              ctx.textAlign = 'left';
              ctx.textBaseline = 'middle';
              const yPos = bar ? bar.y : y.getPixelForValue(index);
              ctx.fillText(text, left + 15, yPos);
            }
          });
        });
        ctx.restore();
      }
    }],
    options: {
      indexAxis: 'y', // Horizontal bars
      responsive: true,
      maintainAspectRatio: false,
      scales: {
        x: {
          grid: {
            color: 'rgba(255, 255, 255, 0.05)',
            drawBorder: false
          },
          ticks: {
            color: '#94a3b8',
            font: {
              family: 'JetBrains Mono',
              size: 11
            }
          },
          title: {
            display: true,
            text: 'Seconds (Lower is Better)',
            color: '#64748b',
            font: {
              family: 'Outfit',
              weight: 'bold'
            }
          }
        },
        y: {
          grid: {
            display: false
          },
          ticks: {
            color: '#f8fafc',
            font: {
              family: 'Outfit',
              size: 13,
              weight: '600'
            }
          }
        }
      },
      plugins: {
        legend: {
          display: false
        },
        tooltip: {
          backgroundColor: '#0f111a',
          titleColor: '#f8fafc',
          bodyColor: '#cbd5e1',
          bodyFont: {
            family: 'JetBrains Mono'
          },
          titleFont: {
            family: 'Outfit',
            weight: 'bold'
          },
          borderColor: 'rgba(124, 58, 237, 0.3)',
          borderWidth: 1,
          padding: 12,
          displayColors: false,
          callbacks: {
            label: function(context) {
              return `Time: ${context.parsed.x} seconds`;
            }
          }
        }
      }
    }
  });
}

function updateChart(driveKey) {
  const currentData = benchmarkData[driveKey];
  
  // Update header text
  document.getElementById('benchmark-title').textContent = currentData.title;
  document.getElementById('benchmark-desc').textContent = currentData.desc;
  
  // Animate chart transition and update properties dynamically
  benchmarkChart.data.labels = currentData.labels;
  benchmarkChart.data.dnfTexts = currentData.dnfTexts;
  benchmarkChart.data.datasets[0].data = currentData.datasets[0].data;
  benchmarkChart.data.datasets[0].backgroundColor = currentData.datasets[0].backgroundColor;
  benchmarkChart.data.datasets[0].borderColor = currentData.datasets[0].borderColor;
  
  benchmarkChart.update();
}

// --- DOM READY INITIALIZATION ---
document.addEventListener('DOMContentLoaded', () => {
  // Initialize Lucide Icons
  createIcons({
    icons: {
      Folder, File, Zap, Cpu, Shield, Layers, Image, HardDrive, Download, 
      ChevronRight, ChevronDown, Trash2, BarChart2, Eye, Copy, ExternalLink,
      Database
    }
  });
  
  // Render Simulator components
  const treeContainer = document.getElementById('sim-tree-root');
  if (treeContainer) {
    treeContainer.appendChild(createTreeDOM(mockData));
  }
  
  const treemapCanvas = document.getElementById('sim-treemap-canvas');
  if (treemapCanvas) {
    renderTreemap(mockData, treemapCanvas, 0, 0, 100, 100, true);
  }
  
  // Re-run icon parser for generated elements
  createIcons({ icons: { Folder, File } });
  
  // Setup Benchmark Tabs
  const benchmarkTabs = document.querySelectorAll('.benchmark-tab');
  benchmarkTabs.forEach(tab => {
    tab.addEventListener('click', () => {
      benchmarkTabs.forEach(t => t.classList.remove('active'));
      tab.classList.add('active');
      
      const target = tab.getAttribute('data-target');
      updateChart(target);
    });
  });
  
  // Init Chart.js
  if (document.getElementById('benchmarkChart')) {
    initChart();
  }
  
  // Setup Guide Tabs
  const guideTabs = document.querySelectorAll('.guide-tab');
  const guidePanes = document.querySelectorAll('.guide-pane');
  guideTabs.forEach(tab => {
    tab.addEventListener('click', () => {
      guideTabs.forEach(t => t.classList.remove('active'));
      guidePanes.forEach(p => p.classList.remove('active'));
      
      tab.classList.add('active');
      const target = tab.getAttribute('data-target');
      document.getElementById(target).classList.add('active');
    });
  });
});
