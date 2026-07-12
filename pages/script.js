document.addEventListener('DOMContentLoaded', () => {
  // Terminal blocks Copy functionality
  const copyBtns = document.querySelectorAll('.copy-btn');

  copyBtns.forEach(btn => {
    btn.addEventListener('click', () => {
      const targetId = btn.dataset.clipboardTarget;
      const codeEl = document.querySelector(targetId);

      if (codeEl) {
        // Extract command lines (ignore $ signs)
        const lines = codeEl.querySelectorAll('.cmd-line');
        const textToCopy = Array.from(lines).map(line => line.textContent).join('\n');

        navigator.clipboard.writeText(textToCopy).then(() => {
          btn.textContent = 'Copied!';
          btn.style.backgroundColor = 'var(--success-color)';
          btn.style.color = '#ffffff';
          btn.style.borderColor = 'transparent';

          setTimeout(() => {
            btn.textContent = 'Copy';
            btn.style.backgroundColor = '';
            btn.style.color = '';
            btn.style.borderColor = '';
          }, 2000);
        }).catch(err => {
          console.error('Failed to copy text: ', err);
        });
      }
    });
  });
});
