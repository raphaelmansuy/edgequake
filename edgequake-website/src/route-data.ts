import { defineRouteMiddleware } from "@astrojs/starlight/route-data";

const DOCS_SYMLINK_PREFIX = "src/content/docs/docs/";
const REPO_EDIT_BASE =
  "https://github.com/raphaelmansuy/edgequake/edit/edgequake-main/docs/";

export const onRequest = defineRouteMiddleware((context) => {
  const { entry } = context.locals.starlightRoute;
  if (!entry.filePath.startsWith(DOCS_SYMLINK_PREFIX)) {
    return;
  }

  const docsRelativePath = entry.filePath.slice(DOCS_SYMLINK_PREFIX.length);
  context.locals.starlightRoute.editUrl = new URL(
    `${REPO_EDIT_BASE}${docsRelativePath}`,
  );
});
