declare module "react-cytoscapejs" {
  import type cytoscape from "cytoscape";
  import type { Component } from "react";

  interface CytoscapeComponentProps {
    elements: cytoscape.ElementDefinition[];
    stylesheet?: cytoscape.StylesheetCSS[];
    style?: React.CSSProperties;
    cy?: (cy: cytoscape.Core) => void;
    layout?: cytoscape.LayoutOptions;
    className?: string;
  }

  export default class CytoscapeComponent extends Component<CytoscapeComponentProps> {}
}
