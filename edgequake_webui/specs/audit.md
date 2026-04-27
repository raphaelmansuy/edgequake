# Site Audit

base: http://localhost:3000

## http://localhost:3000

- title: EdgeQuake - Knowledge Graph RAG Platform
- h1: Dashboard
- screenshot: ./screenshots/01-home.png
- console_messages:
  - info: %cDownload the React DevTools for a better development experience: https://react.dev/link/react-devtools font-weight:bold
  - info: 🌐 i18next is maintained with support from locize.com — consider powering your project with managed localization (AI, CDN, integrations): https://locize.com 💙
  - log: [HMR] connected
  - warning: WebSocket connection to 'ws://localhost:8080/ws/pipeline/progress' failed: WebSocket is closed before the connection is established.
  - warning: [ProgressWebSocket] Connection unavailable - backend may not be running
  - log: [ProgressWebSocket] Backend confirmed connection

## http://localhost:3000/

- title: EdgeQuake - Knowledge Graph RAG Platform
- h1: Dashboard
- screenshot: ./screenshots/02-home.png
- console_messages:
  - info: %cDownload the React DevTools for a better development experience: https://react.dev/link/react-devtools font-weight:bold
  - info: 🌐 i18next is maintained with support from locize.com — consider powering your project with managed localization (AI, CDN, integrations): https://locize.com 💙
  - log: [HMR] connected
  - warning: WebSocket connection to 'ws://localhost:8080/ws/pipeline/progress' failed: WebSocket is closed before the connection is established.
  - warning: [ProgressWebSocket] Connection unavailable - backend may not be running
  - log: [ProgressWebSocket] Backend confirmed connection
  - info: %cDownload the React DevTools for a better development experience: https://react.dev/link/react-devtools font-weight:bold
  - log: [HMR] connected
  - info: 🌐 i18next is maintained with support from locize.com — consider powering your project with managed localization (AI, CDN, integrations): https://locize.com 💙
  - warning: WebSocket connection to 'ws://localhost:8080/ws/pipeline/progress' failed: WebSocket is closed before the connection is established.
  - warning: [ProgressWebSocket] Connection unavailable - backend may not be running
  - log: [ProgressWebSocket] Backend confirmed connection

## http://localhost:3000/graph

- title: EdgeQuake - Knowledge Graph RAG Platform
- h1: (none)
- screenshot: ./screenshots/03-_graph.png
- console_messages:
  - warning: [GraphRenderer] Sigma initialization failed: TypeError: Cannot read properties of null (reading 'blendFunc')
    at Sigma.createWebGLContext (http://localhost:3000/_next/static/chunks/node_modules_0kfqin3._.js:19007:20)
    at new Sigma (http://localhost:3000/_next/static/chunks/node_modules_0kfqin3._.js:17731:15)
    at GraphRenderer.useCallback[initializeGraph] (http://localhost:3000/_next/static/chunks/src_components_graph_0wl~ha7._.js:4175:25)
    at GraphRenderer.useEffect (http://localhost:3000/_next/static/chunks/src_components_graph_0wl~ha7._.js:4484:29)
    at Object.react_stack_bottom_frame (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:15087:22)
    at runWithFiberInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:965:74)
    at commitHookEffectListMount (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:7255:167)
    at commitHookPassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:7290:60)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8689:33)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8750:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8750:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8750:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8750:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8688:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8688:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8738:783)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8730:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8688:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8750:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8688:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8688:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8688:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8738:783)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8730:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8688:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8750:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8688:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8688:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8688:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
  - warning: [GraphRenderer] Sigma initialization failed: TypeError: Cannot read properties of null (reading 'blendFunc')
    at Sigma.createWebGLContext (http://localhost:3000/_next/static/chunks/node_modules_0kfqin3._.js:19007:20)
    at new Sigma (http://localhost:3000/_next/static/chunks/node_modules_0kfqin3._.js:17731:15)
    at GraphRenderer.useCallback[initializeGraph] (http://localhost:3000/_next/static/chunks/src_components_graph_0wl~ha7._.js:4175:25)
    at GraphRenderer.useEffect (http://localhost:3000/_next/static/chunks/src_components_graph_0wl~ha7._.js:4484:29)
    at Object.react_stack_bottom_frame (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:15087:22)
    at runWithFiberInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:965:74)
    at commitHookEffectListMount (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:7255:167)
    at commitHookPassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:7290:60)
    at reconnectPassiveEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8779:17)
    at recursivelyTraverseReconnectPassiveEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8766:13)
    at reconnectPassiveEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8793:17)
    at doubleInvokeEffectsOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:10117:133)
    at runWithFiberInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:965:74)
    at recursivelyTraverseAndDoubleInvokeEffectsInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:10110:79)
    at recursivelyTraverseAndDoubleInvokeEffectsInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:10110:147)
    at recursivelyTraverseAndDoubleInvokeEffectsInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:10110:147)
    at recursivelyTraverseAndDoubleInvokeEffectsInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:10110:147)
    at recursivelyTraverseAndDoubleInvokeEffectsInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:10110:147)
    at recursivelyTraverseAndDoubleInvokeEffectsInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:10110:147)
    at runWithFiberInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:965:131)
    at recursivelyTraverseAndDoubleInvokeEffectsInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:10110:405)
    at recursivelyTraverseAndDoubleInvokeEffectsInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:10110:147)
    at recursivelyTraverseAndDoubleInvokeEffectsInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:10110:147)
    at recursivelyTraverseAndDoubleInvokeEffectsInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:10110:147)
    at recursivelyTraverseAndDoubleInvokeEffectsInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:10110:147)
    at recursivelyTraverseAndDoubleInvokeEffectsInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:10110:147)
    at recursivelyTraverseAndDoubleInvokeEffectsInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:10110:147)
    at runWithFiberInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:965:131)
    at recursivelyTraverseAndDoubleInvokeEffectsInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:10110:405)
    at recursivelyTraverseAndDoubleInvokeEffectsInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:10110:147)
    at recursivelyTraverseAndDoubleInvokeEffectsInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:10110:147)
    at recursivelyTraverseAndDoubleInvokeEffectsInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:10110:147)
    at recursivelyTraverseAndDoubleInvokeEffectsInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:10110:147)
    at recursivelyTraverseAndDoubleInvokeEffectsInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:10110:147)
    at recursivelyTraverseAndDoubleInvokeEffectsInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:10110:147)
    at recursivelyTraverseAndDoubleInvokeEffectsInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:10110:147)
    at recursivelyTraverseAndDoubleInvokeEffectsInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:10110:147)
    at recursivelyTraverseAndDoubleInvokeEffectsInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:10110:147)
    at recursivelyTraverseAndDoubleInvokeEffectsInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:10110:147)
    at recursivelyTraverseAndDoubleInvokeEffectsInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:10110:147)
    at recursivelyTraverseAndDoubleInvokeEffectsInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:10110:147)
    at recursivelyTraverseAndDoubleInvokeEffectsInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:10110:147)
    at recursivelyTraverseAndDoubleInvokeEffectsInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:10110:147)
    at recursivelyTraverseAndDoubleInvokeEffectsInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:10110:147)
    at recursivelyTraverseAndDoubleInvokeEffectsInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:10110:147)
    at recursivelyTraverseAndDoubleInvokeEffectsInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:10110:147)
    at recursivelyTraverseAndDoubleInvokeEffectsInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:10110:147)
    at recursivelyTraverseAndDoubleInvokeEffectsInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:10110:147)
    at recursivelyTraverseAndDoubleInvokeEffectsInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:10110:147)
    at recursivelyTraverseAndDoubleInvokeEffectsInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:10110:147)
  - warning: [GraphRenderer] Sigma initialization failed: TypeError: Cannot read properties of null (reading 'blendFunc')
    at Sigma.createWebGLContext (http://localhost:3000/_next/static/chunks/node_modules_0kfqin3._.js:19007:20)
    at new Sigma (http://localhost:3000/_next/static/chunks/node_modules_0kfqin3._.js:17731:15)
    at GraphRenderer.useCallback[initializeGraph] (http://localhost:3000/_next/static/chunks/src_components_graph_0wl~ha7._.js:4175:25)
    at GraphRenderer.useEffect (http://localhost:3000/_next/static/chunks/src_components_graph_0wl~ha7._.js:4484:29)
    at Object.react_stack_bottom_frame (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:15087:22)
    at runWithFiberInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:965:74)
    at commitHookEffectListMount (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:7255:167)
    at commitHookPassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:7290:60)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8689:33)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8750:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8750:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8750:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8750:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8688:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8688:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8738:783)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8730:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8688:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8750:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8688:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8688:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8688:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8738:783)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8730:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8688:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8750:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8688:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8688:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8688:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
  - warning: [GraphRenderer] Sigma initialization failed: TypeError: Cannot read properties of null (reading 'blendFunc')
    at Sigma.createWebGLContext (http://localhost:3000/_next/static/chunks/node_modules_0kfqin3._.js:19007:20)
    at new Sigma (http://localhost:3000/_next/static/chunks/node_modules_0kfqin3._.js:17731:15)
    at GraphRenderer.useCallback[initializeGraph] (http://localhost:3000/_next/static/chunks/src_components_graph_0wl~ha7._.js:4175:25)
    at GraphRenderer.useEffect (http://localhost:3000/_next/static/chunks/src_components_graph_0wl~ha7._.js:4484:29)
    at Object.react_stack_bottom_frame (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:15087:22)
    at runWithFiberInDEV (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:965:74)
    at commitHookEffectListMount (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:7255:167)
    at commitHookPassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:7290:60)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8689:33)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8750:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8750:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8750:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8750:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8688:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8688:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8738:783)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8730:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8688:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8750:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8688:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8688:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8688:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8738:783)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8730:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8688:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8750:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8688:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8688:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
    at commitPassiveMountOnFiber (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8688:17)
    at recursivelyTraversePassiveMountEffects (http://localhost:3000/_next/static/chunks/node_modules_next_dist_compiled_react-dom_058-ah~._.js:8676:316)
  - log: [Fast Refresh] rebuilding
  - log: [ProgressWebSocket] Backend confirmed connection
  - log: [Fast Refresh] done in 414ms
  - log: [Fast Refresh] rebuilding
  - log: [Fast Refresh] done in 160ms
  - log: [Fast Refresh] rebuilding
  - log: [Fast Refresh] done in 146ms
  - log: [Fast Refresh] rebuilding
  - log: [Fast Refresh] done in 539ms

## http://localhost:3000/graph

- error: page.$$eval: Target page, context or browser has been closed

## http://localhost:3000/documents

- error: page.goto: Target page, context or browser has been closed

## http://localhost:3000/pipeline

- error: page.goto: Target page, context or browser has been closed

## http://localhost:3000/query

- error: page.goto: Target page, context or browser has been closed

## http://localhost:3000/workspace

- error: page.goto: Target page, context or browser has been closed

## http://localhost:3000/costs

- error: page.goto: Target page, context or browser has been closed

## http://localhost:3000/knowledge

- error: page.goto: Target page, context or browser has been closed

## http://localhost:3000/api-explorer

- error: page.goto: Target page, context or browser has been closed

## http://localhost:3000/settings

- error: page.goto: Target page, context or browser has been closed

## http://localhost:3000/documents?id=5de7da87-09eb-46e8-9678-58498f0d4f65

- error: page.goto: Target page, context or browser has been closed
