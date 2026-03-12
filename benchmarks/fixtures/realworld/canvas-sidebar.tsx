// L tier - Inspired by excalidraw Sidebar with layer management
import { useState, useMemo, useCallback, useRef, useEffect } from 'react';

interface Layer {
  id: string;
  name: string;
  visible: boolean;
  locked: boolean;
  opacity: number;
  elements: number;
}

interface SidebarProps {
  layers: Layer[];
  activeLayerId: string;
  onLayerSelect: (id: string) => void;
  onLayerToggleVisible: (id: string) => void;
  onLayerToggleLock: (id: string) => void;
  onLayerRename: (id: string, name: string) => void;
  onLayerReorder: (fromIndex: number, toIndex: number) => void;
  onLayerDelete: (id: string) => void;
  onLayerAdd: () => void;
  onLayerDuplicate: (id: string) => void;
  onLayerOpacity: (id: string, opacity: number) => void;
}

type Tab = 'layers' | 'properties' | 'history';

export function CanvasSidebar({
  layers,
  activeLayerId,
  onLayerSelect,
  onLayerToggleVisible,
  onLayerToggleLock,
  onLayerRename,
  onLayerReorder,
  onLayerDelete,
  onLayerAdd,
  onLayerDuplicate,
  onLayerOpacity,
}: SidebarProps) {
  const [activeTab, setActiveTab] = useState<Tab>('layers');
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editName, setEditName] = useState('');
  const [searchQuery, setSearchQuery] = useState('');
  const [dragIndex, setDragIndex] = useState<number | null>(null);
  const [collapsed, setCollapsed] = useState(false);
  const editInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (editingId && editInputRef.current) {
      editInputRef.current.focus();
      editInputRef.current.select();
    }
  }, [editingId]);

  const filteredLayers = useMemo(() => {
    if (!searchQuery) return layers;
    const q = searchQuery.toLowerCase();
    return layers.filter((l) => l.name.toLowerCase().includes(q));
  }, [layers, searchQuery]);

  const activeLayer = useMemo(
    () => layers.find((l) => l.id === activeLayerId),
    [layers, activeLayerId]
  );

  const stats = useMemo(() => ({
    totalElements: layers.reduce((sum, l) => sum + l.elements, 0),
    visibleLayers: layers.filter((l) => l.visible).length,
    lockedLayers: layers.filter((l) => l.locked).length,
  }), [layers]);

  const startEditing = useCallback((layer: Layer) => {
    setEditingId(layer.id);
    setEditName(layer.name);
  }, []);

  const commitEdit = useCallback(() => {
    if (editingId && editName.trim()) {
      onLayerRename(editingId, editName.trim());
    }
    setEditingId(null);
  }, [editingId, editName, onLayerRename]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'Enter') commitEdit();
      if (e.key === 'Escape') setEditingId(null);
    },
    [commitEdit]
  );

  const handleDragStart = useCallback((index: number) => {
    setDragIndex(index);
  }, []);

  const handleDragOver = useCallback((e: React.DragEvent, index: number) => {
    e.preventDefault();
    if (dragIndex !== null && dragIndex !== index) {
      onLayerReorder(dragIndex, index);
      setDragIndex(index);
    }
  }, [dragIndex, onLayerReorder]);

  const handleDragEnd = useCallback(() => {
    setDragIndex(null);
  }, []);

  if (collapsed) {
    return (
      <div className="w-10 bg-white border-l flex flex-col items-center py-2">
        <button onClick={() => setCollapsed(false)} className="text-gray-500 hover:text-gray-700">
          ◀
        </button>
      </div>
    );
  }

  return (
    <div className="w-64 bg-white border-l flex flex-col h-full">
      <div className="flex items-center justify-between px-3 py-2 border-b">
        <div className="flex gap-1">
          {(['layers', 'properties', 'history'] as Tab[]).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={`px-2 py-1 text-xs rounded ${
                activeTab === tab ? 'bg-blue-100 text-blue-700' : 'text-gray-500 hover:bg-gray-100'
              }`}
            >
              {tab.charAt(0).toUpperCase() + tab.slice(1)}
            </button>
          ))}
        </div>
        <button onClick={() => setCollapsed(true)} className="text-gray-400 hover:text-gray-600">
          ▶
        </button>
      </div>

      {activeTab === 'layers' && (
        <>
          <div className="px-3 py-2 border-b">
            <input
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Search layers..."
              className="w-full text-xs border rounded px-2 py-1"
            />
          </div>

          <div className="flex-1 overflow-y-auto">
            {filteredLayers.map((layer, index) => (
              <div
                key={layer.id}
                draggable
                onDragStart={() => handleDragStart(index)}
                onDragOver={(e) => handleDragOver(e, index)}
                onDragEnd={handleDragEnd}
                onClick={() => onLayerSelect(layer.id)}
                className={`flex items-center px-3 py-2 text-sm cursor-pointer border-b ${
                  layer.id === activeLayerId ? 'bg-blue-50' : 'hover:bg-gray-50'
                } ${dragIndex === index ? 'opacity-50' : ''}`}
              >
                <button
                  onClick={(e) => { e.stopPropagation(); onLayerToggleVisible(layer.id); }}
                  className="mr-2 text-xs"
                >
                  {layer.visible ? '👁' : '○'}
                </button>

                {editingId === layer.id ? (
                  <input
                    ref={editInputRef}
                    value={editName}
                    onChange={(e) => setEditName(e.target.value)}
                    onBlur={commitEdit}
                    onKeyDown={handleKeyDown}
                    className="flex-1 text-xs border rounded px-1"
                  />
                ) : (
                  <span
                    onDoubleClick={() => startEditing(layer)}
                    className={`flex-1 truncate ${!layer.visible ? 'text-gray-400' : ''}`}
                  >
                    {layer.name}
                  </span>
                )}

                <span className="text-xs text-gray-400 ml-1">{layer.elements}</span>

                <button
                  onClick={(e) => { e.stopPropagation(); onLayerToggleLock(layer.id); }}
                  className="ml-1 text-xs"
                >
                  {layer.locked ? '🔒' : '🔓'}
                </button>
              </div>
            ))}
          </div>

          <div className="px-3 py-2 border-t">
            <div className="flex justify-between items-center">
              <div className="text-xs text-gray-500">
                {stats.totalElements} elements · {stats.visibleLayers}/{layers.length} visible
              </div>
              <div className="flex gap-1">
                <button onClick={onLayerAdd} className="text-xs px-2 py-1 bg-blue-500 text-white rounded">
                  +
                </button>
                {activeLayer && (
                  <>
                    <button
                      onClick={() => onLayerDuplicate(activeLayerId)}
                      className="text-xs px-2 py-1 border rounded"
                    >
                      ⧉
                    </button>
                    <button
                      onClick={() => onLayerDelete(activeLayerId)}
                      className="text-xs px-2 py-1 border rounded text-red-500"
                      disabled={layers.length <= 1}
                    >
                      🗑
                    </button>
                  </>
                )}
              </div>
            </div>

            {activeLayer && (
              <div className="mt-2">
                <label className="text-xs text-gray-500">Opacity</label>
                <input
                  type="range"
                  min={0}
                  max={100}
                  value={activeLayer.opacity * 100}
                  onChange={(e) => onLayerOpacity(activeLayerId, parseInt(e.target.value) / 100)}
                  className="w-full"
                />
                <span className="text-xs text-gray-400">{Math.round(activeLayer.opacity * 100)}%</span>
              </div>
            )}
          </div>
        </>
      )}

      {activeTab === 'properties' && (
        <div className="p-3 text-sm text-gray-500">
          {activeLayer ? (
            <div className="space-y-2">
              <div><strong>Name:</strong> {activeLayer.name}</div>
              <div><strong>Elements:</strong> {activeLayer.elements}</div>
              <div><strong>Visible:</strong> {activeLayer.visible ? 'Yes' : 'No'}</div>
              <div><strong>Locked:</strong> {activeLayer.locked ? 'Yes' : 'No'}</div>
              <div><strong>Opacity:</strong> {Math.round(activeLayer.opacity * 100)}%</div>
            </div>
          ) : (
            <p>Select a layer to view properties</p>
          )}
        </div>
      )}

      {activeTab === 'history' && (
        <div className="p-3 text-sm text-gray-500 text-center">
          History panel (not implemented)
        </div>
      )}
    </div>
  );
}
