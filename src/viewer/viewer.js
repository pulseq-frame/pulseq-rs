// Shared JS for both viewers. Each viewer also emits a small inline <script>
// containing data-dependent plot calls or shape-data assignments; that script
// runs *before* this one, so any globals it defines are visible here.

// Used by viewer_raw's per-shape Plotly.newPlot calls.
var common = { margin: { t: 10, r: 10, b: 30, l: 45 }, showlegend: false };

// Used by viewer_structured's shape-popup hover behavior. The inline data
// script populates these maps with sample arrays keyed by shape id.
window.shapes = window.shapes || {};
window.cshapes_re = window.cshapes_re || {};
window.cshapes_im = window.cshapes_im || {};

document.addEventListener('DOMContentLoaded', function () {
  // Shape-popup hover (viewer_structured). No-op in viewer_raw because there
  // are no .shape-link elements there.
  var layout = {
    margin: { t: 10, r: 10, b: 30, l: 40 },
    showlegend: false,
    width: 480,
    height: 240,
    xaxis: { title: 'sample' }
  };
  var clayout = Object.assign({}, layout, { showlegend: true });
  var config = { responsive: false, displaylogo: false, displayModeBar: false };
  var registry = document.getElementById('shape-registry');

  function ensurePlotted(plotDiv, shapeId) {
    if (!plotDiv.dataset.rendered) {
      Plotly.newPlot(
        plotDiv,
        [{ y: window.shapes[shapeId], mode: 'lines', line: { width: 1.2 } }],
        layout,
        config
      );
      plotDiv.dataset.rendered = '1';
    }
  }

  function ensureComplexPlotted(plotDiv, shapeId) {
    if (!plotDiv.dataset.rendered) {
      Plotly.newPlot(
        plotDiv,
        [
          { y: window.cshapes_re[shapeId], mode: 'lines', line: { width: 1.2 }, name: 'real' },
          { y: window.cshapes_im[shapeId], mode: 'lines', line: { width: 1.2 }, name: 'imag' }
        ],
        clayout,
        config
      );
      plotDiv.dataset.rendered = '1';
    }
  }

  document.querySelectorAll('.shape-link').forEach(function (link) {
    var shapeId = link.dataset.shape;
    var cshapeId = link.dataset.cshape;
    var plotDiv, plotter;
    if (shapeId) {
      plotDiv = document.getElementById('shape-plot-' + shapeId);
      plotter = function () { ensurePlotted(plotDiv, shapeId); };
    } else if (cshapeId) {
      plotDiv = document.getElementById('cshape-plot-' + cshapeId);
      plotter = function () { ensureComplexPlotted(plotDiv, cshapeId); };
    }
    var popup = link.querySelector('.shape-popup');
    if (!plotDiv || !popup) return;
    link.addEventListener('mouseenter', function () {
      popup.appendChild(plotDiv);
      plotter();
    });
    link.addEventListener('mouseleave', function () {
      if (registry) registry.appendChild(plotDiv);
    });
  });
});
