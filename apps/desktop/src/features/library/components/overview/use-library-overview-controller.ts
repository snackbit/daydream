import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type MouseEvent,
} from 'react';
import { useNavigate } from '@tanstack/react-router';
import type {
  LibraryRecordingSortKey,
  LibraryRecordingSummary,
  LibraryRecordingViewMode,
} from '@daydream/shared';
import {
  deleteLibraryRecording,
  deleteLibraryRecordings,
  listLibraryRecordings,
  prepareLibraryRecordingThumbnail,
  renameLibraryRecording,
} from '@/features/library/library-client';
import { formatTimestamp } from '@/features/library/library-format';
import {
  errorMessage,
  isRecordingDeletable,
  isRecordingRenameable,
  isRecordingSelectable,
  keepThumbnailPaths,
  selectedRange,
  sortRecordings,
  uniqueIds,
} from './library-overview-utils';

export const RECORDING_TITLE_MAX_CHARS = 120;

export interface LibraryOverviewDeleteTarget {
  ids: string[];
  label: string;
}

export interface LibraryOverviewRenameTarget {
  fallbackTitle: string;
  id: string;
}

export interface LibraryOverviewModel {
  deleteTarget: LibraryOverviewDeleteTarget | null;
  error: string | null;
  librarySizeBytes: number;
  recordings: LibraryRecordingSummary[];
  renameSaving: boolean;
  renameTarget: LibraryOverviewRenameTarget | null;
  renameTitle: string;
  selectedIdSet: Set<string>;
  selectedIds: string[];
  selectedSizeBytes: number;
  sortKey: LibraryRecordingSortKey;
  sortedRecordings: LibraryRecordingSummary[];
  status: 'error' | 'idle' | 'loading' | 'ready';
  thumbnailPaths: Record<string, string>;
  viewMode: LibraryRecordingViewMode;
}

export interface LibraryOverviewActions {
  closeDeleteDialog: () => void;
  closeRenameDialog: () => void;
  confirmDelete: () => Promise<void>;
  confirmRename: () => Promise<void>;
  openRecording: (recording: LibraryRecordingSummary) => void;
  refresh: () => Promise<void>;
  requestRecordingDelete: (recording: LibraryRecordingSummary) => void;
  requestRecordingRename: (recording: LibraryRecordingSummary) => void;
  requestSelectedDelete: () => void;
  selectRecording: (
    recording: LibraryRecordingSummary,
    event: MouseEvent<HTMLElement>,
  ) => void;
  setRenameTitle: (title: string) => void;
  setSortKey: (sortKey: LibraryRecordingSortKey) => void;
  setViewMode: (viewMode: LibraryRecordingViewMode) => void;
}

export interface LibraryOverviewController {
  actions: LibraryOverviewActions;
  model: LibraryOverviewModel;
}

export function useLibraryOverviewController(): LibraryOverviewController {
  const navigate = useNavigate();
  const [deleteTarget, setDeleteTarget] =
    useState<LibraryOverviewDeleteTarget | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [renameSaving, setRenameSaving] = useState(false);
  const [renameTarget, setRenameTarget] =
    useState<LibraryOverviewRenameTarget | null>(null);
  const [renameTitle, setRenameTitle] = useState('');
  const thumbnailLoadRunId = useRef(0);
  const [selectionAnchorId, setSelectionAnchorId] = useState<string | null>(
    null,
  );
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const [recordings, setRecordings] = useState<LibraryRecordingSummary[]>([]);
  const [sortKey, setSortKey] = useState<LibraryRecordingSortKey>('started');
  const [status, setStatus] = useState<'error' | 'idle' | 'loading' | 'ready'>(
    'idle',
  );
  const [thumbnailPaths, setThumbnailPaths] = useState<Record<string, string>>(
    {},
  );
  const [viewMode, setViewMode] = useState<LibraryRecordingViewMode>('grid');

  const sortedRecordings = useMemo(
    () => sortRecordings(recordings, sortKey),
    [recordings, sortKey],
  );
  const recordingsById = useMemo(
    () => new Map(recordings.map((recording) => [recording.id, recording])),
    [recordings],
  );
  const selectedIdSet = useMemo(() => new Set(selectedIds), [selectedIds]);
  const selectedRecordings = useMemo(
    () =>
      selectedIds
        .map((recordingId) => recordingsById.get(recordingId))
        .filter((recording): recording is LibraryRecordingSummary =>
          Boolean(recording),
        ),
    [recordingsById, selectedIds],
  );
  const selectedSizeBytes = useMemo(
    () =>
      selectedRecordings.reduce(
        (total, recording) => total + recording.sizeBytes,
        0,
      ),
    [selectedRecordings],
  );
  const librarySizeBytes = useMemo(
    () =>
      recordings.reduce((total, recording) => total + recording.sizeBytes, 0),
    [recordings],
  );

  const loadOverviewThumbnails = useCallback(
    async (nextRecordings: LibraryRecordingSummary[], runId: number) => {
      for (const recording of nextRecordings) {
        if (recording.availability !== 'available') {
          continue;
        }
        const thumbnail = await prepareLibraryRecordingThumbnail({
          recordingId: recording.id,
        }).catch(() => null);
        if (thumbnailLoadRunId.current !== runId) {
          return;
        }
        const thumbnailPath = thumbnail?.path;
        if (thumbnail?.error === null && thumbnailPath) {
          setThumbnailPaths((current) => ({
            ...current,
            [thumbnail.recordingId]: thumbnailPath,
          }));
        }
      }
    },
    [],
  );

  const refresh = useCallback(async () => {
    setError(null);
    setStatus('loading');
    try {
      const nextRecordings = await listLibraryRecordings();
      const nextRecordingIds = new Set(
        nextRecordings
          .filter(isRecordingSelectable)
          .map((recording) => recording.id),
      );
      setRecordings(nextRecordings);
      setThumbnailPaths((current) =>
        keepThumbnailPaths(current, nextRecordings),
      );
      setSelectedIds((current) =>
        current.filter((recordingId) => nextRecordingIds.has(recordingId)),
      );
      setStatus('ready');
      const nextThumbnailRunId = thumbnailLoadRunId.current + 1;
      thumbnailLoadRunId.current = nextThumbnailRunId;
      void loadOverviewThumbnails(nextRecordings, nextThumbnailRunId);
    } catch (loadError) {
      setError(errorMessage(loadError));
      setStatus('error');
    }
  }, [loadOverviewThumbnails]);

  useEffect(() => {
    void Promise.resolve().then(refresh);
    return () => {
      thumbnailLoadRunId.current += 1;
    };
  }, [refresh]);

  const confirmDelete = useCallback(async () => {
    if (!deleteTarget) {
      return;
    }
    setError(null);
    try {
      if (deleteTarget.ids.length === 1) {
        await deleteLibraryRecording({ recordingId: deleteTarget.ids[0]! });
      } else {
        await deleteLibraryRecordings({ recordingIds: deleteTarget.ids });
      }
      const deletedIds = new Set(deleteTarget.ids);
      setRecordings((current) =>
        current.filter((recording) => !deletedIds.has(recording.id)),
      );
      setSelectedIds((current) =>
        current.filter((recordingId) => !deletedIds.has(recordingId)),
      );
      setDeleteTarget(null);
    } catch (deleteError) {
      setError(errorMessage(deleteError));
    }
  }, [deleteTarget]);

  const confirmRename = useCallback(async () => {
    if (!renameTarget) {
      return;
    }
    const trimmedTitle = renameTitle.trim();
    setError(null);
    setRenameSaving(true);
    try {
      const detail = await renameLibraryRecording({
        recordingId: renameTarget.id,
        title: trimmedTitle.length > 0 ? trimmedTitle : null,
      });
      setRecordings((current) =>
        current.map((recording) =>
          recording.id === detail.id
            ? {
                ...recording,
                title: detail.title,
              }
            : recording,
        ),
      );
      setRenameTarget(null);
      setRenameTitle('');
    } catch (renameError) {
      setError(errorMessage(renameError));
    } finally {
      setRenameSaving(false);
    }
  }, [renameTarget, renameTitle]);

  const selectRecording = useCallback(
    (recording: LibraryRecordingSummary, event: MouseEvent<HTMLElement>) => {
      if (!isRecordingSelectable(recording)) {
        return;
      }

      const modifierPressed = event.ctrlKey || event.metaKey;
      if (event.shiftKey && selectionAnchorId) {
        const nextRange = selectedRange(
          sortedRecordings,
          selectionAnchorId,
          recording.id,
        );
        setSelectedIds((current) =>
          modifierPressed ? uniqueIds([...current, ...nextRange]) : nextRange,
        );
        return;
      }

      if (modifierPressed) {
        setSelectedIds((current) =>
          current.includes(recording.id)
            ? current.filter((id) => id !== recording.id)
            : [...current, recording.id],
        );
        setSelectionAnchorId(recording.id);
        return;
      }

      setSelectedIds([recording.id]);
      setSelectionAnchorId(recording.id);
    },
    [selectionAnchorId, sortedRecordings],
  );

  const targetForRecordingDelete = useCallback(
    (
      recording: LibraryRecordingSummary,
    ): LibraryOverviewDeleteTarget | null => {
      const selectedRecordingIds = selectedIdSet.has(recording.id)
        ? selectedIds
        : [recording.id];
      const deletableRecordingIds = selectedRecordingIds.filter(
        (recordingId) => {
          const selectedRecording = recordingsById.get(recordingId);

          return selectedRecording
            ? isRecordingDeletable(selectedRecording)
            : false;
        },
      );

      if (deletableRecordingIds.length === 0) {
        return null;
      }

      return {
        ids: deletableRecordingIds,
        label:
          deletableRecordingIds.length === 1
            ? '1 recording'
            : `${deletableRecordingIds.length} recordings`,
      };
    },
    [recordingsById, selectedIdSet, selectedIds],
  );

  const requestRecordingDelete = useCallback(
    (recording: LibraryRecordingSummary) => {
      const target = targetForRecordingDelete(recording);
      if (target) {
        setDeleteTarget(target);
      }
    },
    [targetForRecordingDelete],
  );

  const requestSelectedDelete = useCallback(() => {
    if (selectedIds.length === 0) {
      return;
    }
    setDeleteTarget({
      ids: selectedIds,
      label:
        selectedIds.length === 1
          ? '1 recording'
          : `${selectedIds.length} recordings`,
    });
  }, [selectedIds]);

  const requestRecordingRename = useCallback(
    (recording: LibraryRecordingSummary) => {
      if (!isRecordingRenameable(recording)) {
        return;
      }
      setRenameTarget({
        fallbackTitle: formatTimestamp(recording.startedAtMs),
        id: recording.id,
      });
      setRenameTitle(recording.title ?? '');
    },
    [],
  );

  const openRecording = useCallback(
    (recording: LibraryRecordingSummary) => {
      if (recording.availability !== 'available') {
        return;
      }

      void navigate({
        params: { recordingId: recording.id },
        to: '/library/$recordingId',
      });
    },
    [navigate],
  );

  const closeDeleteDialog = useCallback(() => setDeleteTarget(null), []);
  const closeRenameDialog = useCallback(() => {
    setRenameTarget(null);
    setRenameTitle('');
  }, []);

  return {
    actions: {
      closeDeleteDialog,
      closeRenameDialog,
      confirmDelete,
      confirmRename,
      openRecording,
      refresh,
      requestRecordingDelete,
      requestRecordingRename,
      requestSelectedDelete,
      selectRecording,
      setRenameTitle,
      setSortKey,
      setViewMode,
    },
    model: {
      deleteTarget,
      error,
      librarySizeBytes,
      recordings,
      renameSaving,
      renameTarget,
      renameTitle,
      selectedIdSet,
      selectedIds,
      selectedSizeBytes,
      sortKey,
      sortedRecordings,
      status,
      thumbnailPaths,
      viewMode,
    },
  };
}
