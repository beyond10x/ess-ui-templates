import type { CSSProperties, ReactElement } from 'react';

/** The document `ess-ui data` writes; see schema/ess-ui-presentation.schema.json. */
export interface EssPresentationData {
  format: 'ess-ui-presentation/1';
  generator: string;
  system: string;
  version: string;
  title: string;
  ir_sha256: string;
  document_title: string;
  body_class: string;
  body: string;
  model: Record<string, unknown>;
  sim: Record<string, unknown>;
  unrendered: string[];
}

export interface EssPresentationProps {
  data: EssPresentationData;
  theme?: 'light' | 'dark';
  height?: string | number;
  title?: string;
  className?: string;
  style?: CSSProperties;
}

export function EssPresentation(props: EssPresentationProps): ReactElement;
export function compose(data: EssPresentationData, assets: { css: string; js: string; headJs: string }, opts?: { theme?: 'light' | 'dark' }): string;
export default EssPresentation;
