  namespace ProjectManagement.Core.State;

  /// <summary>
  /// State machine for app startup flow.
  /// Ensures proper sequencing: Identity → Server → WebSocket → UI
  /// </summary>
  public enum AppStartupState
  {
      /// <summary>Initial state - checking if desktop mode</summary>
      Initializing,

      /// <summary>Desktop mode - waiting for server to start</summary>
      WaitingForServer,

      /// <summary>Server ready, checking for existing user identity</summary>
      CheckingIdentity,

      /// <summary>First launch - user must register</summary>
      NeedsRegistration,

      /// <summary>Identity loaded, connecting WebSocket</summary>
      ConnectingWebSocket,

      /// <summary>All systems ready - show main UI</summary>
      Ready,

      /// <summary>Recoverable error - show retry option</summary>
      Error,

      /// <summary>WebSocket disconnected - attempting reconnect</summary>
      Reconnecting
  }