"""
Core functionality for RustRay Python SDK.
"""

import os
import sys
import time
import threading
from typing import Any, List, Optional, Union

from .data import ObjectRef, ObjectStore, DataContext
from .grpc_client import GrpcClient

_global_client = None
_global_context = None
_init_lock = threading.Lock()

def init(
    address: Optional[str] = None,
    *,
    namespace: Optional[str] = None,
    include_dashboard: bool = True,
    ignore_reinit_error: bool = True,
    _redis_password: Optional[str] = None,
    **kwargs
) -> None:
    """
    Initialize the RustRay runtime.

    Args:
        address: The address of the RustRay head node. If None, will start a local cluster.
        namespace: Namespace to place all RustRay objects in.
        include_dashboard: If True, start the web dashboard.
        ignore_reinit_error: If True, ignore reinit error.
        **kwargs: Other parameters.
    """
    global _global_client, _global_context

    with _init_lock:
        if _global_client is not None:
            if ignore_reinit_error:
                return
            raise RuntimeError("RustRay has already been initialized.")

        if address is None:
            address = "localhost:8000"

        # Create gRPC client
        client = GrpcClient(address)
        
        # Connect to RustRay backend
        config = {
            "namespace": namespace or "default",
            "include_dashboard": str(include_dashboard).lower(),
            **kwargs
        }
        client.connect(config)
        
        # Initialize global client and context
        _global_client = client
        _global_context = DataContext(namespace=namespace)

def shutdown() -> None:
    """
    Shutdown the RustRay runtime.
    """
    global _global_client, _global_context

    with _init_lock:
        if _global_client is None:
            return

        _global_client.close()
        _global_client = None
        _global_context = None

def get(
    object_refs: Union[ObjectRef, List[ObjectRef]],
    timeout: Optional[float] = None
) -> Any:
    """
    Get a remote object or a list of remote objects.

    Args:
        object_refs: Object ref(s) to get.
        timeout: Timeout in seconds.

    Returns:
        The object(s) referenced by the object refs.
    """
    if _global_client is None:
        raise RuntimeError("RustRay has not been initialized. Call ray.init() first.")

    single = False
    if isinstance(object_refs, ObjectRef):
        single = True
        object_refs = [object_refs]

    results = []
    deadline = time.time() + (timeout or float("inf"))

    for ref in object_refs:
        while True:
            result = _global_client.get_task_result(ref.id)
            if result is not None or time.time() > deadline:
                break
            time.sleep(0.1)
        
        if result is None:
            raise TimeoutError(f"Timeout waiting for object {ref.id}")
        results.append(result)

    if single:
        return results[0]
    return results

def wait(
    object_refs: List[ObjectRef],
    *,
    num_returns: Optional[int] = None,
    timeout: Optional[float] = None,
    fetch_local: bool = True,
) -> tuple:
    """
    Wait for a list of object refs to be locally available.

    Args:
        object_refs: List of object refs to wait for.
        num_returns: Number of objects that should be returned.
        timeout: Timeout in seconds.
        fetch_local: If True, fetch objects to local store.

    Returns:
        A tuple of (ready_refs, remaining_refs).
    """
    if _global_client is None:
        raise RuntimeError("RustRay has not been initialized. Call ray.init() first.")

    if num_returns is None:
        num_returns = len(object_refs)

    ready = []
    remaining = list(object_refs)
    deadline = time.time() + (timeout or float("inf"))

    while len(ready) < num_returns and time.time() <= deadline:
        for ref in remaining[:]:
            result = _global_client.get_task_result(ref.id)
            if result is not None:
                ready.append(ref)
                remaining.remove(ref)
                if len(ready) >= num_returns:
                    break
        if len(ready) < num_returns:
            time.sleep(0.1)

    return ready, remaining

def put(value: Any) -> ObjectRef:
    """
    Store an object in the object store.

    Args:
        value: The value to store.

    Returns:
        An ObjectRef referring to the stored object.
    """
    if _global_client is None:
        raise RuntimeError("RustRay has not been initialized. Call ray.init() first.")

    object_id = _global_client.put_object(value)
    return ObjectRef(object_id)

def get_context() -> DataContext:
    """
    Get the current data context.

    Returns:
        The current DataContext object.
    """
    if _global_context is None:
        raise RuntimeError("RustRay has not been initialized. Call ray.init() first.")

    return _global_context 