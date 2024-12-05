"""
Remote function and class decorators for RustRay.
"""

import inspect
import functools
from typing import Any, Callable, Optional, Union

from .data import ObjectRef
from .core import _global_client

def remote(
    function_or_class: Optional[Callable] = None,
    *,
    num_cpus: Optional[float] = None,
    num_gpus: Optional[float] = None,
    memory: Optional[int] = None,
    resources: Optional[dict] = None,
    max_calls: Optional[int] = None,
    max_retries: int = 3,
    runtime_env: Optional[dict] = None,
) -> Union[Callable, "RemoteFunction"]:
    """
    Decorator for creating remote functions or actors.

    Args:
        function_or_class: Function or class to be made remote.
        num_cpus: Number of CPUs required.
        num_gpus: Number of GPUs required.
        memory: Amount of memory required in bytes.
        resources: Custom resource requirements.
        max_calls: Maximum number of calls before worker restart.
        max_retries: Maximum number of retries on failure.
        runtime_env: Runtime environment for the remote function.

    Returns:
        A remote function or actor class.
    """
    def _make_remote(function_or_class):
        if inspect.isclass(function_or_class):
            return _make_remote_class(
                function_or_class,
                num_cpus=num_cpus,
                num_gpus=num_gpus,
                memory=memory,
                resources=resources,
                max_calls=max_calls,
                max_retries=max_retries,
                runtime_env=runtime_env,
            )
        else:
            return _make_remote_function(
                function_or_class,
                num_cpus=num_cpus,
                num_gpus=num_gpus,
                memory=memory,
                resources=resources,
                max_calls=max_calls,
                max_retries=max_retries,
                runtime_env=runtime_env,
            )

    if function_or_class is None:
        return _make_remote
    return _make_remote(function_or_class)

class RemoteFunction:
    """
    Wrapper class for remote functions.
    """
    def __init__(
        self,
        function: Callable,
        num_cpus: Optional[float] = None,
        num_gpus: Optional[float] = None,
        memory: Optional[int] = None,
        resources: Optional[dict] = None,
        max_calls: Optional[int] = None,
        max_retries: int = 3,
        runtime_env: Optional[dict] = None,
    ):
        self._function = function
        self._num_cpus = num_cpus
        self._num_gpus = num_gpus
        self._memory = memory
        self._resources = resources or {}
        self._max_calls = max_calls
        self._max_retries = max_retries
        self._runtime_env = runtime_env or {}

        functools.update_wrapper(self, function)

    def remote(self, *args, **kwargs) -> ObjectRef:
        """
        Execute the remote function.

        Returns:
            ObjectRef: A reference to the remote object.
        """
        if _global_client is None:
            raise RuntimeError(
                "RustRay has not been initialized. Call rustray.init() first."
            )

        return _global_client.submit_task(
            self._function,
            args=args,
            kwargs=kwargs,
            num_cpus=self._num_cpus,
            num_gpus=self._num_gpus,
            memory=self._memory,
            resources=self._resources,
            max_retries=self._max_retries,
            runtime_env=self._runtime_env,
        )

    def __call__(self, *args, **kwargs):
        """
        Execute the function locally for debugging.
        """
        return self._function(*args, **kwargs)

def _make_remote_function(
    function: Callable,
    num_cpus: Optional[float] = None,
    num_gpus: Optional[float] = None,
    memory: Optional[int] = None,
    resources: Optional[dict] = None,
    max_calls: Optional[int] = None,
    max_retries: int = 3,
    runtime_env: Optional[dict] = None,
) -> RemoteFunction:
    """
    Convert a function into a remote function.
    """
    return RemoteFunction(
        function,
        num_cpus=num_cpus,
        num_gpus=num_gpus,
        memory=memory,
        resources=resources,
        max_calls=max_calls,
        max_retries=max_retries,
        runtime_env=runtime_env,
    )

def _make_remote_class(
    cls: type,
    num_cpus: Optional[float] = None,
    num_gpus: Optional[float] = None,
    memory: Optional[int] = None,
    resources: Optional[dict] = None,
    max_calls: Optional[int] = None,
    max_retries: int = 3,
    runtime_env: Optional[dict] = None,
) -> type:
    """
    Convert a class into a remote actor class.
    """
    # This will be handled by the actor module
    from .actor import make_actor_class
    return make_actor_class(
        cls,
        num_cpus=num_cpus,
        num_gpus=num_gpus,
        memory=memory,
        resources=resources,
        max_calls=max_calls,
        max_retries=max_retries,
        runtime_env=runtime_env,
    ) 