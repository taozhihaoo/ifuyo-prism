import os

class Service:
    """服务说明。"""

    def run(self, 任务):
        return os.name + 任务

def build_service():
    return Service()
