"""
Actor system implementation for RustRay.
"""

import inspect
import threading
from typing import Any, Callable, Dict, Optional, Type

from .data import ObjectRef
from .core import _global_client

def actor(
    cls: Optional[Type] = None,
    *,
    num_cpus: Optional[float] = None,
    num_gpus: Optional[float] = None,
    memory: Optional[int] = None,
    resources: Optional[dict] = None,
    max_concurrency: int = 1,
    max_restarts: int = -1,
    lifetime: Optional[str] = None,
    namespace: Optional[str] = None,
) -> Type:
    """
    Decorator for creating actor classes.

    Args:
        cls: The class to convert into an actor class.
        num_cpus: Number of CPUs required by the actor.
        num_gpus: Number of GPUs required by the actor.
        memory: Amount of memory required in bytes.
        resources: Custom resource requirements.
        max_concurrency: Maximum number of concurrent calls to this actor.
        max_restarts: Maximum number of restarts before failing (-1 means infinite).
        lifetime: Actor's lifetime ("detached" or "non_detached").
        namespace: Namespace for the actor.

    Returns:
        A RemoteActorClass.
    """
    def _make_actor(cls):
        return make_actor_class(
            cls,
            num_cpus=num_cpus,
            num_gpus=num_gpus,
            memory=memory,
            resources=resources,
            max_concurrency=max_concurrency,
            max_restarts=max_restarts,
            lifetime=lifetime,
            namespace=namespace,
        )

    if cls is None:
        return _make_actor
    return _make_actor(cls)

class ActorMethod:
    """
    Wrapper class for actor methods.
    """
    def __init__(self, actor_handle, method_name: str):
        self._actor_handle = actor_handle
        self._method_name = method_name

    def remote(self, *args, **kwargs) -> ObjectRef:
        """
        Execute the actor method remotely.

        Returns:
            ObjectRef: A reference to the result.
        """
        if _global_client is None:
            raise RuntimeError(
                "RustRay has not been initialized. Call rustray.init() first."
            )

        return _global_client.submit_actor_task(
            self._actor_handle,
            self._method_name,
            args=args,
            kwargs=kwargs,
        )

class ActorHandle:
    """
    Handle to a remote actor.
    """
    def __init__(self, actor_id: str, class_name: str):
        self._actor_id = actor_id
        self._class_name = class_name
        self._method_cache: Dict[str, ActorMethod] = {}
        self._lock = threading.Lock()

    def __getattr__(self, name: str) -> ActorMethod:
        """
        Get a reference to an actor method.
        """
        with self._lock:
            if name not in self._method_cache:
                self._method_cache[name] = ActorMethod(self, name)
            return self._method_cache[name]

    @property
    def actor_id(self) -> str:
        """
        Get the unique ID of this actor.
        """
        return self._actor_id

class ActorClass:
    """
    Class that converts an actor class into a remote actor class.
    """
    def __init__(
        self,
        cls: Type,
        num_cpus: Optional[float] = None,
        num_gpus: Optional[float] = None,
        memory: Optional[int] = None,
        resources: Optional[dict] = None,
        max_concurrency: int = 1,
        max_restarts: int = -1,
        lifetime: Optional[str] = None,
        namespace: Optional[str] = None,
    ):
        self._cls = cls
        self._num_cpus = num_cpus
        self._num_gpus = num_gpus
        self._memory = memory
        self._resources = resources or {}
        self._max_concurrency = max_concurrency
        self._max_restarts = max_restarts
        self._lifetime = lifetime
        self._namespace = namespace

    def remote(self, *args, **kwargs) -> ActorHandle:
        """
        Create a new remote actor.

        Returns:
            ActorHandle: Handle to the remote actor.
        """
        if _global_client is None:
            raise RuntimeError(
                "RustRay has not been initialized. Call rustray.init() first."
            )

        actor_id = _global_client.create_actor(
            self._cls,
            args=args,
            kwargs=kwargs,
            num_cpus=self._num_cpus,
            num_gpus=self._num_gpus,
            memory=self._memory,
            resources=self._resources,
            max_concurrency=self._max_concurrency,
            max_restarts=self._max_restarts,
            lifetime=self._lifetime,
            namespace=self._namespace,
        )

        return ActorHandle(actor_id, self._cls.__name__)

def make_actor_class(
    cls: Type,
    num_cpus: Optional[float] = None,
    num_gpus: Optional[float] = None,
    memory: Optional[int] = None,
    resources: Optional[dict] = None,
    max_concurrency: int = 1,
    max_restarts: int = -1,
    lifetime: Optional[str] = None,
    namespace: Optional[str] = None,
) -> ActorClass:
    """
    Convert a Python class into an actor class.
    """
    # Validate the class
    if not inspect.isclass(cls):
        raise TypeError("The @actor decorator can only be applied to classes.")

    # Create and return actor class
    return ActorClass(
        cls,
        num_cpus=num_cpus,
        num_gpus=num_gpus,
        memory=memory,
        resources=resources,
        max_concurrency=max_concurrency,
        max_restarts=max_restarts,
        lifetime=lifetime,
        namespace=namespace,
    ) 