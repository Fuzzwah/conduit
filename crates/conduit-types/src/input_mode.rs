/// Input mode for the application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputMode {
    /// Normal mode - input focused
    #[default]
    Normal,
    /// File viewer tab is active
    FileViewer,
    /// Selecting agent for new tab
    SelectingAgent,
    /// Scrolling through chat history
    Scrolling,
    /// Navigating sidebar
    SidebarNavigation,
    /// Adding a repository (custom path)
    AddingRepository,
    /// Selecting model for current session
    SelectingModel,
    /// Selecting reasoning effort for current session
    SelectingReasoning,
    /// Selecting theme
    SelectingTheme,
    /// Selecting enabled providers
    SelectingProviders,
    /// Setting base projects directory
    SettingBaseDir,
    /// Picking a project from the list
    PickingProject,
    /// Showing a confirmation dialog
    Confirming,
    /// Removing a project (showing spinner)
    RemovingProject,
    /// Cloning a remote repository (showing spinner)
    CloningRepository,
    /// Creating a new workspace (showing progress dialog)
    CreatingWorkspace,
    /// Showing an error dialog
    ShowingError,
    /// Command mode (typing :command)
    Command,
    /// Showing help dialog
    ShowingHelp,
    /// Importing a session from external agent
    ImportingSession,
    /// Settings menu dialog is open
    SettingsMenu,
    /// Command palette is open
    CommandPalette,
    /// Slash command menu is open
    SlashMenu,
    /// Missing tool dialog is open
    MissingTool,
    /// Editing global workspace defaults
    WorkspaceDefaults,
    /// Renaming a project
    RenamingProject,
    /// Managing MCP settings for a project
    ProjectMcp,
    /// Browsing local filesystem to pick a source file (step 1 of add-file flow)
    FilePickerSource,
    /// Browsing repository directories to pick a copy destination (step 2 of add-file flow)
    FilePickerDest,
    /// Displaying SCP command for uploading a file from a remote workstation
    ScpCommand,
    /// Editing queued messages inline
    QueueEditing,
    /// File mention autocomplete (@filename) is active
    FileMention,
    /// Syncing the base repo with the remote before showing the issue picker
    SyncingRemote,
    /// Picking a GitHub issue to link to the new workspace
    SelectingIssue,
    /// Picking an OpenSpec change to link to the new workspace
    SelectingSpec,
    /// Picking a spec-kit (specify) spec to link to the new workspace
    SelectingSpecifySpec,
    /// Keybindings editor dialog is open (list + filter mode)
    KeybindingsEditor,
    /// Waiting for a keypress to capture as a new binding
    KeybindingsEditorCapture,
    /// Work Complete dialog is open (commit / push / PR / archive flow)
    WorkCompleting,
}

/// View mode for the main content area
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    /// Standard chat view
    #[default]
    Chat,
    /// Raw events debug view
    RawEvents,
}
