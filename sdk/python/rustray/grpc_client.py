"""
gRPC client implementation for RustRay Python SDK.
"""

import uuid
import grpc
import pickle
from typing import Any, Dict, Optional

from . import rustray_pb2
from . import rustray_pb2_grpc

class GrpcClient:
    """
    gRPC client for communicating with RustRay backend.
    """
    def __init__(self, address: str):
        self.channel = grpc.insecure_channel(address)
        self.stub = rustray_pb2_grpc.RustRayStub(self.channel)
        self.session_id = None
        self.client_id = str(uuid.uuid4())

    def connect(self, config: Optional[Dict[str, str]] = None) -> None:
        """
        Connect to RustRay backend.
        """
        request = rustray_pb2.ConnectRequest(
            client_id=self.client_id,
            config=config or {}
        )
        response = self.stub.Connect(request)
        self.session_id = response.session_id

    def submit_task(
        self,
        function_name: str,
        args: tuple,
        resources: Optional[Dict[str, str]] = None
    ) -> str:
        """
        Submit a task to RustRay backend.
        """
        request = rustray_pb2.TaskRequest(
            session_id=self.session_id,
            function_name=function_name,
            args=pickle.dumps(args),
            resources=resources or {}
        )
        response = self.stub.SubmitTask(request)
        return response.task_id

    def get_task_result(self, task_id: str) -> Any:
        """
        Get task result from RustRay backend.
        """
        request = rustray_pb2.TaskResultRequest(
            session_id=self.session_id,
            task_id=task_id
        )
        response = self.stub.GetTaskResult(request)
        
        if response.status == rustray_pb2.TaskResultResponse.FAILED:
            raise RuntimeError(response.error)
        elif response.status == rustray_pb2.TaskResultResponse.COMPLETED:
            return pickle.loads(response.result)
        else:
            return None

    def create_actor(
        self,
        class_name: str,
        args: tuple,
        resources: Optional[Dict[str, str]] = None
    ) -> str:
        """
        Create an actor on RustRay backend.
        """
        request = rustray_pb2.CreateActorRequest(
            session_id=self.session_id,
            class_name=class_name,
            args=pickle.dumps(args),
            resources=resources or {}
        )
        response = self.stub.CreateActor(request)
        return response.actor_id

    def call_actor_method(
        self,
        actor_id: str,
        method_name: str,
        args: tuple
    ) -> str:
        """
        Call an actor method on RustRay backend.
        """
        request = rustray_pb2.CallActorRequest(
            session_id=self.session_id,
            actor_id=actor_id,
            method_name=method_name,
            args=pickle.dumps(args)
        )
        response = self.stub.CallActorMethod(request)
        return response.task_id

    def put_object(self, data: Any, data_type: Optional[str] = None) -> str:
        """
        Store an object in RustRay's object store.
        """
        request = rustray_pb2.PutObjectRequest(
            session_id=self.session_id,
            data=pickle.dumps(data),
            data_type=data_type or type(data).__name__
        )
        response = self.stub.PutObject(request)
        return response.object_id

    def get_object(self, object_id: str) -> Any:
        """
        Get an object from RustRay's object store.
        """
        request = rustray_pb2.GetObjectRequest(
            session_id=self.session_id,
            object_id=object_id
        )
        response = self.stub.GetObject(request)
        return pickle.loads(response.data)

    def delete_object(self, object_id: str) -> bool:
        """
        Delete an object from RustRay's object store.
        """
        request = rustray_pb2.DeleteObjectRequest(
            session_id=self.session_id,
            object_id=object_id
        )
        response = self.stub.DeleteObject(request)
        return response.success

    def close(self) -> None:
        """
        Close the gRPC channel.
        """
        self.channel.close() 