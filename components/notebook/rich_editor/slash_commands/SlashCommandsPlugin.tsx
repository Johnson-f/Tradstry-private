'use client';

import { useCallback, useMemo, useState, type CSSProperties, type RefObject } from 'react';
import { createPortal } from 'react-dom';
import {
  LexicalTypeaheadMenuPlugin,
  MenuOption,
  useBasicTypeaheadTriggerMatch,
} from '@lexical/react/LexicalTypeaheadMenuPlugin';
import { useLexicalComposerContext } from '@lexical/react/LexicalComposerContext';
import {
  $getSelection,
  $isRangeSelection,
  type LexicalEditor,
  type TextNode,
} from 'lexical';
import { slashCommandConfigs } from './commands';

class SlashCommandOption extends MenuOption {
  title: string;
  description?: string;
  keywords: string[];
  execute: (editor: LexicalEditor) => void;

  constructor(configKey: string, config: { title: string; description?: string; keywords?: string[]; execute: (editor: LexicalEditor) => void }) {
    super(configKey);
    this.title = config.title;
    this.description = config.description;
    this.keywords = config.keywords ?? [];
    this.execute = config.execute;
  }

  matches(query: string): boolean {
    const lowered = query.toLowerCase();
    return (
      this.title.toLowerCase().includes(lowered) ||
      this.keywords.some((keyword) => keyword.toLowerCase().includes(lowered))
    );
  }
}

// Each item is approximately 52px (py-2 = 8px*2 + content ~36px)
const ITEM_HEIGHT = 52;
const MAX_VISIBLE_ITEMS = 4;

function renderMenu(
  anchorElementRef: RefObject<HTMLElement | null>,
  {
    options,
    selectedIndex,
    selectOptionAndCleanUp,
    setHighlightedIndex,
  }: {
    options: SlashCommandOption[];
    selectedIndex: number | null;
    selectOptionAndCleanUp: (option: SlashCommandOption) => void;
    setHighlightedIndex: (index: number) => void;
  },
) {
  const anchorElement = anchorElementRef.current;
  if (!anchorElement || options.length === 0) {
    return null;
  }

  const { top, left, height } = anchorElement.getBoundingClientRect();
  const style: CSSProperties = {
    position: 'absolute',
    top: top + height + window.scrollY + 8,
    left: left + window.scrollX,
    minWidth: 220,
  };

  const maxHeight = ITEM_HEIGHT * MAX_VISIBLE_ITEMS;
  const needsScroll = options.length > MAX_VISIBLE_ITEMS;

  return createPortal(
    <div
      className="z-50 rounded-md border bg-popover shadow-md"
      style={{
        ...style,
        maxHeight: needsScroll ? maxHeight : 'auto',
        overflow: 'hidden',
      }}
    >
      <div
        className="overflow-y-auto"
        style={{ maxHeight: needsScroll ? maxHeight : 'auto' }}
      >
        {options.map((option, index) => {
          const isSelected = selectedIndex === index;
          return (
            <button
              key={option.key}
              type="button"
              className={`flex w-full flex-col items-start px-3 py-2 text-left text-sm hover:bg-muted ${
                isSelected ? 'bg-muted' : ''
              }`}
              onMouseEnter={() => setHighlightedIndex(index)}
              onMouseDown={(event) => {
                event.preventDefault();
                selectOptionAndCleanUp(option);
              }}
            >
              <span className="font-medium">{option.title}</span>
              {option.description ? (
                <span className="text-xs text-muted-foreground">{option.description}</span>
              ) : null}
            </button>
          );
        })}
      </div>
    </div>,
    document.body,
  );
}

function removeSlashCommandText(textNode: TextNode | null, matchingString: string) {
  if (!textNode) {
    return;
  }

  const selection = $getSelection();
  if (!$isRangeSelection(selection)) {
    return;
  }

  const anchorOffset = selection.anchor.offset;
  const slashLength = 1;
  const start = anchorOffset - matchingString.length - slashLength;
  const end = anchorOffset;

  if (start >= 0 && end > start) {
    textNode.spliceText(start, end - start, '');
  }
}

export function SlashCommandsPlugin() {
  const [editor] = useLexicalComposerContext();
  const [query, setQuery] = useState<string | null>(null);
  const triggerFn = useBasicTypeaheadTriggerMatch('/', { minLength: 0, maxLength: 32 });

  const baseOptions = useMemo(
    () =>
      slashCommandConfigs.map(
        (config) =>
          new SlashCommandOption(config.key, {
            title: config.title,
            description: config.description,
            keywords: config.keywords,
            execute: config.execute,
          }),
      ),
    [],
  );

  const options = useMemo(() => {
    if (!query) {
      return baseOptions;
    }
    const normalized = query.trim().toLowerCase();
    if (normalized.length === 0) {
      return baseOptions;
    }
    return baseOptions.filter((option) => option.matches(normalized));
  }, [baseOptions, query]);

  const onSelectOption = useCallback(
    (option: SlashCommandOption, textNode: TextNode | null, closeMenu: () => void, matchingString: string) => {
      editor.update(() => {
        removeSlashCommandText(textNode, matchingString);
      });

      // Execute command after a small delay to ensure text removal completes
      setTimeout(() => {
        option.execute(editor);
        closeMenu();
      }, 0);
    },
    [editor],
  );

  const onQueryChange = useCallback((matchingString: string | null) => {
    setQuery(matchingString);
  }, []);

  return (
    <LexicalTypeaheadMenuPlugin
      onQueryChange={onQueryChange}
      onSelectOption={onSelectOption}
      options={options}
      triggerFn={triggerFn}
      menuRenderFn={(anchorElementRef, itemProps) => renderMenu(anchorElementRef, itemProps)}
      preselectFirstItem
    />
  );
}

export default SlashCommandsPlugin;
