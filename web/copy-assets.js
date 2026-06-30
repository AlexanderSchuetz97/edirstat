const fs = require('fs');
const path = require('path');

const projectRoot = __dirname;
const repoRoot = path.resolve(projectRoot, '..');

const targets = [
  {
    src: path.join(repoRoot, 'assets/img/icon.ico'),
    dest: path.join(projectRoot, 'static/favicon.ico')
  },
  {
    src: path.join(repoRoot, 'assets/img/logo-nosubtext-transparent.svg'),
    dest: path.join(projectRoot, 'static/assets/logo-nosubtext-transparent.svg')
  }
];

for (const target of targets) {
  const destDir = path.dirname(target.dest);
  if (!fs.existsSync(destDir)) {
    fs.mkdirSync(destDir, { recursive: true });
  }
  fs.copyFileSync(target.src, target.dest);
  console.log(`Copied ${path.relative(repoRoot, target.src)} -> ${path.relative(projectRoot, target.dest)}`);
}
